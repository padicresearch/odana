use std::collections::BTreeSet;
use std::fs::File;
use std::iter;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use colored::Colorize;
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use libp2p::core::connection::{Connected, ConnectedPoint};
use libp2p::core::multiaddr::Protocol;
use libp2p::core::network::Peer;
use libp2p::core::ProtocolName;
use libp2p::core::transport::upgrade::Version;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::futures::{AsyncRead, AsyncWrite, AsyncWriteExt, SinkExt, StreamExt};
use libp2p::futures::future::err;
use libp2p::gossipsub::{
    Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity, Sha256Topic,
    ValidationMode,
};
use libp2p::identify::IdentifyConfig;
use libp2p::kad::{GetClosestPeersError, GetClosestPeersOk, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::kad::record::store::MemoryStore;
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::NetworkBehaviour;
use libp2p::noise::{AuthenticKeypair, NoiseConfig, X25519Spec};
use libp2p::request_response::{
    ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseConfig,
    RequestResponseEvent, RequestResponseMessage,
};
use libp2p::swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent};
use libp2p::tcp::TokioTcpConfig;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use codec::{Decoder, Encoder};
use crypto::{generate_pow_from_pub_key, SHA256};
use primitive_types::{Compact, H256, U192};
use tracing::{error, info, trace, warn};
use types::block::{Block, BlockHeader};
use types::config::{EnvironmentConfig, NodeIdentityConfig};
use types::Hash;
use types::tx::Transaction;

use crate::identity::*;
use crate::message::*;
use crate::peer_manager::PeerList;

pub mod identity;
pub mod message;
pub mod peer_manager;

trait P2pEnvironment {
    fn node_identity(&self) -> NodeIdentity;
    fn p2p_address(&self) -> Multiaddr;
    fn topic(&self) -> Sha256Topic;
    fn p2p_pow_target(&self) -> Compact;
}

impl P2pEnvironment for EnvironmentConfig {
    fn node_identity(&self) -> NodeIdentity {
        let file = File::open(&self.identity_file).unwrap();
        let identity_config: NodeIdentityConfig = serde_json::from_reader(file).unwrap();
        return NodeIdentity::from_config(identity_config).unwrap();
        //NodeIdentity::generate(self.network.max_difficulty_compact())
    }

    fn p2p_address(&self) -> Multiaddr {
        Multiaddr::empty()
            .with(Protocol::Ip4(self.host.parse().unwrap()))
            .with(Protocol::Tcp(self.p2p_port))
    }

    fn topic(&self) -> Sha256Topic {
        Sha256Topic::new(self.network)
    }

    fn p2p_pow_target(&self) -> Compact {
        self.network.max_difficulty_compact()
    }
}

async fn config_network(
    node_identity: NodeIdentity,
    p2p_to_node: UnboundedSender<PeerMessage>,
    peer_list: Arc<PeerList>,
    pow_target: Compact
) -> Result<Swarm<ChainNetworkBehavior>> {
    let auth_keys = libp2p::noise::Keypair::<X25519Spec>::new()
        .into_authentic(&node_identity.identity_keys())
        .expect("cannot create auth keys");

    let transport = TokioTcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(libp2p::mplex::MplexConfig::new())
        .boxed();

    let network_topic = Sha256Topic::new("testnet");

    let mdns = Mdns::new(Default::default())
        .await
        .expect("Cannot create mdns");
    let mut cfg = KademliaConfig::default();
    cfg.set_query_timeout(Duration::from_secs(5 * 60));
    let kad = Kademlia::with_config(node_identity.peer_id().clone(), MemoryStore::new(node_identity.peer_id().clone()), cfg);

    let max_transmit_size = 500;
    let config = GossipsubConfigBuilder::default()
        .max_transmit_size(max_transmit_size)
        .protocol_id_prefix("tuchain")
        .validation_mode(ValidationMode::Permissive)
        .build()
        .expect("Failed to create Gossip sub network");

    let local_peer_id = node_identity.peer_id().clone();
    let public_ip = public_ip::addr_v4().await.unwrap();
    let public_address = Multiaddr::empty().with(Protocol::Ip4(public_ip)).with(Protocol::Tcp(9020)).with(Protocol::P2p(local_peer_id.into()));


    let mut behaviour = ChainNetworkBehavior {
        gossipsub: Gossipsub::new(
            MessageAuthenticity::Author(node_identity.peer_id().clone()),
            config,
        )
            .expect("Failed to create Gossip sub network"),
        mdns,
        kad,
        requestresponse: RequestResponse::new(
            ChainP2pExchangeCodec,
            iter::once((ChainP2pExchangeProtocol, ProtocolSupport::Full)),
            RequestResponseConfig::default(),
        ),
        p2p_to_node,
        topic: network_topic.clone(),
        node: node_identity.to_p2p_node(),
        public_address,
        peers: peer_list,
        pow_target
    };

    behaviour.gossipsub.subscribe(&network_topic);

    let swarm = SwarmBuilder::new(transport, behaviour, node_identity.peer_id().clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    Ok(swarm)
}

pub async fn start_p2p_server(
    node_identity: NodeIdentity,
    mut node_to_p2p: UnboundedReceiver<PeerMessage>,
    p2p_to_node: UnboundedSender<PeerMessage>,
    peer_arg: Option<String>,
    peer_list: Arc<PeerList>,
    pow_target: Compact
) -> Result<()> {
    let mut swarm = config_network(node_identity.clone(), p2p_to_node, peer_list, pow_target).await?;

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9020".parse()?)
        .expect("Error connecting to p2p");

    if let Some(to_dial) = peer_arg {
        let addr: Multiaddr = to_dial.parse()?;
        let peer_id = match addr.iter().last() {
            Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
            _ => anyhow::bail!("Expect peer multiaddr to contain peer ID."),
        };
        swarm.behaviour_mut().kad.add_address(&peer_id, addr.with(Protocol::P2p(peer_id.into())));
        swarm.behaviour_mut().kad.get_closest_peers(node_identity.peer_id().clone());
        //swarm.dial(addr.with(Protocol::P2p(peer_id.into())))?;
    }

    tokio::task::spawn(async move {
        loop {
            tokio::select! {
            msg = node_to_p2p.recv() => {handle_publish_message(msg, &mut swarm).await}
            event = swarm.select_next_some() => {handle_swam_event(event, &mut swarm).await}}
        }
    });
    Ok(())
}

async fn handle_publish_message(msg: Option<PeerMessage>, swarm: &mut Swarm<ChainNetworkBehavior>) {
    if let Some(msg) = msg {
        if let Ok(encoded_msg) = msg.encode() {
            let network_topic = swarm.behaviour_mut().topic.clone();
            swarm
                .behaviour_mut()
                .gossipsub
                .publish(network_topic, encoded_msg);
        } else {
            println!("Failed to encode message {:?}", msg)
        }
    }
}

async fn handle_swam_event<T: std::fmt::Debug>(
    event: SwarmEvent<OutEvent, T>,
    swarm: &mut Swarm<ChainNetworkBehavior>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            let local_peer_id = *swarm.local_peer_id();
            info!(
                "Node listening on {}",
                format!("{}", address.with(Protocol::P2p(local_peer_id.into()))).blue()
            );
        }
        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Message {
                                                      propagation_source,
                                                      message_id,
                                                      message,
                                                  })) => {
            if let Ok(peer_message) = PeerMessage::decode(&message.data) {
                swarm.behaviour_mut().p2p_to_node.send(peer_message);
            }
        }

        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Unsubscribed {
                                                      peer_id, topic
                                                  })) => {
            if topic == swarm.behaviour_mut().topic.hash() {
                swarm.disconnect_peer_id(peer_id);
            }
        }

        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
            for (peer, addr) in list {
                info!(peer = ?addr, "New Peer discovered");
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
            }
        }
        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
            for (peer, addr) in list {
                warn!(peer = ?addr, "Peer expired");
                if !swarm.behaviour_mut().mdns.has_node(&peer) {
                    swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                }
            }
        }

        SwarmEvent::Behaviour(OutEvent::Kademlia(KademliaEvent::OutboundQueryCompleted { id, result: QueryResult::GetClosestPeers(result), stats })) => {
            match result {
                Ok(ok) => {
                    info!(list = ?ok.peers,"Found Peers");
                }
                Err(_) => {}
            }
        }

        SwarmEvent::Behaviour(OutEvent::Kademlia(KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer })) => {
            if is_new_peer {
                for address in addresses.iter() {
                    println!("Sending Ack to {}", address);
                    let addr = swarm.behaviour_mut().public_address.clone();
                    let request_id = swarm
                        .behaviour_mut()
                        .requestresponse
                        .send_request(&peer, PeerMessage::Ack(addr));
                    swarm.behaviour().peers.add_potential_peer(
                        peer,
                        request_id,
                    );
                    swarm.behaviour().peers.set_peer_address(
                        peer,
                        address.clone(),
                    );
                }
            }
        }

        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message {
                                                            peer,
                                                            message,
                                                        })) => match message {
            RequestResponseMessage::Request {
                request_id,
                request,
                channel,
            } => match &request {
                PeerMessage::Ack(addr) => {
                    let chain_network = swarm.behaviour_mut();
                    chain_network.kad.add_address(&peer, addr.clone());
                    chain_network.peers.set_peer_address(peer, addr.clone());
                    println!("Sending ReAck to {}", addr);
                    chain_network.requestresponse.send_response(
                        channel,
                        PeerMessage::ReAck(ReAckMessage::new(chain_network.node, vec![])),
                    );
                }
                _ => {}
            },

            RequestResponseMessage::Response {
                request_id,
                response,
            } => match &response {
                PeerMessage::ReAck(msg) => {
                    match swarm
                        .behaviour()
                        .peers
                        .promote_peer(&peer, request_id, msg.node_info, swarm.behaviour().pow_target) {
                        Ok(_) => {
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                            info!(peer = ?&peer, peer_node_info = ?msg.node_info, stats = ?swarm.behaviour().peers.stats(),"Connected to new peer");
                        }
                        Err(error) => {
                            warn!(peer = ?&peer, error = ?error,"Failed to promote peer");
                            swarm.disconnect_peer_id(peer);
                        }
                    }
                }
                _ => {}
            },
        },

        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint: ConnectedPoint::Dialer { address }, ..
        } => {
            let addr = swarm.behaviour_mut().public_address.clone();
            let request_id = swarm
                .behaviour_mut()
                .requestresponse
                .send_request(&peer_id, PeerMessage::Ack(addr));
            swarm.behaviour().peers.add_potential_peer(
                peer_id,
                request_id,
            );
            swarm.behaviour().peers.set_peer_address(
                peer_id,
                address.clone(),
            );
            info!(peer = ?address,"Connection established");
        }
        SwarmEvent::ConnectionClosed {
            endpoint: ConnectedPoint::Dialer { address }, cause, peer_id, ..
        } => {
            if let Some(cause) = cause {
                swarm.behaviour_mut().peers.remove_peer(&peer_id);
                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                swarm.behaviour_mut().kad.remove_peer(&peer_id);
                warn!(peer = ?peer_id, address = ?address, cause = ?cause, "Connection closed");
            }
        }
        _ => {}
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
struct ChainNetworkBehavior {
    gossipsub: Gossipsub,
    mdns: Mdns,
    kad: Kademlia<MemoryStore>,
    requestresponse: RequestResponse<ChainP2pExchangeCodec>,
    #[behaviour(ignore)]
    p2p_to_node: UnboundedSender<PeerMessage>,
    #[behaviour(ignore)]
    topic: Sha256Topic,
    #[behaviour(ignore)]
    node: PeerNode,
    #[behaviour(ignore)]
    public_address: Multiaddr,
    #[behaviour(ignore)]
    peers: Arc<PeerList>,
    #[behaviour(ignore)]
    pow_target: Compact,
}

#[derive(Debug)]
enum OutEvent {
    Gossipsub(GossipsubEvent),
    RequestResponse(RequestResponseEvent<PeerMessage, PeerMessage>),
    Mdns(MdnsEvent),
    Kademlia(KademliaEvent),
}

impl From<MdnsEvent> for OutEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}

impl From<GossipsubEvent> for OutEvent {
    fn from(v: GossipsubEvent) -> Self {
        Self::Gossipsub(v)
    }
}

impl From<KademliaEvent> for OutEvent {
    fn from(v: KademliaEvent) -> Self {
        Self::Kademlia(v)
    }
}

impl From<RequestResponseEvent<PeerMessage, PeerMessage>> for OutEvent {
    fn from(v: RequestResponseEvent<PeerMessage, PeerMessage>) -> Self {
        Self::RequestResponse(v)
    }
}

#[derive(Debug, Clone)]
struct ChainP2pExchangeProtocol;

#[derive(Debug, Clone)]
struct ChainP2pExchangeCodec;

impl ProtocolName for ChainP2pExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/tuchain-network/1".as_bytes()
    }
}

#[async_trait]
impl RequestResponseCodec for ChainP2pExchangeCodec {
    type Protocol = ChainP2pExchangeProtocol;
    type Request = PeerMessage;
    type Response = PeerMessage;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let data = read_length_prefixed(io, 1_000_000).await?;
        if data.is_empty() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let message: Result<PeerMessage> = PeerMessage::decode(&data);
        let message = match message {
            Ok(message) => message,
            Err(_) => return Err(std::io::ErrorKind::Unsupported.into()),
        };
        Ok(message)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let data = read_length_prefixed(io, 1_000_000).await?;
        if data.is_empty() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let message: Result<PeerMessage> = PeerMessage::decode(&data);
        let message = match message {
            Ok(message) => message,
            Err(_) => return Err(std::io::ErrorKind::Unsupported.into()),
        };
        Ok(message)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, req.encode().unwrap()).await?;
        io.close().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, res.encode().unwrap()).await?;
        io.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn account_to_node_id() {}
}
