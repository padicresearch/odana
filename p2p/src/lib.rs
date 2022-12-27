use std::io::ErrorKind;
//use std::borrow::BorrowMut;
//use std::fs::File;
use std::iter;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use colored::Colorize;
use libp2p::core::connection::ConnectedPoint;
use libp2p::core::multiaddr::Protocol;
use libp2p::core::transport::upgrade::Version;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::core::ProtocolName;
pub use libp2p::core::{Multiaddr, PeerId};
use libp2p::futures::{AsyncRead, AsyncWrite, AsyncWriteExt, StreamExt};
use libp2p::gossipsub::{
    Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity, Sha256Topic,
    ValidationMode,
};
use libp2p::{Swarm, Transport};
//use libp2p::gossipsub::error::PublishError;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::noise::{NoiseConfig, X25519Spec};
use libp2p::request_response::{
    ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseConfig,
    RequestResponseEvent, RequestResponseMessage,
};
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::tcp::TokioTcpConfig;
use libp2p::NetworkBehaviour;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use codec::{Decodable, Encodable};
use primitive_types::{Compact, U256};
use tracing::{debug, info, warn};
use traits::Blockchain;
use types::config::EnvironmentConfig;

use crate::identity::*;
use crate::message::*;
use crate::peer_manager::NetworkState;
use crate::request_handler::RequestHandler;

pub mod identity;
pub mod message;
pub mod peer_manager;
pub mod request_handler;
pub mod util;

trait P2pEnvironment {
    fn p2p_address(&self) -> Multiaddr;
    fn topic(&self) -> Sha256Topic;
    fn p2p_pow_target(&self) -> Compact;
}

impl P2pEnvironment for EnvironmentConfig {
    fn p2p_address(&self) -> Multiaddr {
        Multiaddr::empty()
            .with(Protocol::Ip4(self.host().parse().unwrap()))
            .with(Protocol::Tcp(self.p2p_port()))
    }

    fn topic(&self) -> Sha256Topic {
        Sha256Topic::new(self.network())
    }

    fn p2p_pow_target(&self) -> Compact {
        self.network().max_difficulty_compact()
    }
}

async fn config_network(
    node_identity: NodeIdentity,
    p2p_to_node: UnboundedSender<Msg>,
    network_state: Arc<NetworkState>,
    pow_target: U256,
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
    let kad = Kademlia::with_config(
        *node_identity.peer_id(),
        MemoryStore::new(*node_identity.peer_id()),
        cfg,
    );

    let max_transmit_size = 1_000_000;
    let config = GossipsubConfigBuilder::default()
        .max_transmit_size(max_transmit_size)
        .protocol_id_prefix("tuchain")
        .idle_timeout(Duration::from_secs(3600))
        .validation_mode(ValidationMode::Permissive)
        .build()
        .expect("Failed to create Gossip sub network");

    let mut behaviour = ChainNetworkBehavior {
        gossipsub: Gossipsub::new(
            MessageAuthenticity::Author(*node_identity.peer_id()),
            config,
        )
        .expect("Failed to create Gossip sub network"),
        mdns,
        kad,
        requestresponse: RequestResponse::new(
            ChainP2pExchangeCodec,
            iter::once((ChainP2pExchangeProtocol, ProtocolSupport::Full)),
            RequestResponseConfig::default()
                .set_connection_keep_alive(Duration::from_secs(3600))
                .clone(),
        ),
        p2p_to_node,
        topic: network_topic.clone(),
        node: node_identity.to_p2p_node(),
        state: network_state,
        pow_target,
    };

    behaviour.gossipsub.subscribe(&network_topic)?;

    let swarm = SwarmBuilder::new(transport, behaviour, *node_identity.peer_id())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    Ok(swarm)
}

async fn handle_send_message_to_peer(
    swarm: &mut Swarm<ChainNetworkBehavior>,
    peer_id: String,
    message: Msg,
) -> Result<()> {
    let peer = PeerId::from_str(&peer_id).unwrap();
    let _ = swarm
        .behaviour_mut()
        .requestresponse
        .send_request(&peer, message);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn start_p2p_server(
    config: Arc<EnvironmentConfig>,
    node_identity: NodeIdentity,
    mut node_to_p2p: UnboundedReceiver<NodeToPeerMessage>,
    p2p_to_node: UnboundedSender<Msg>,
    peer_arg: Vec<String>,
    pow_target: U256,
    network_state: Arc<NetworkState>,
    blockchain: Arc<dyn Blockchain>,
    request_handler: Arc<RequestHandler>,
) -> Result<()> {
    let mut swarm = config_network(
        node_identity.clone(),
        p2p_to_node,
        network_state.clone(),
        pow_target,
    )
    .await?;

    Swarm::listen_on(
        &mut swarm,
        format!("/ip4/{}/tcp/{}", config.host, config.p2p_port).parse()?,
    )
    .expect("Error connecting to p2p");

    for to_dial in peer_arg {
        let addr: Multiaddr = to_dial.parse()?;
        let peer_id = match addr.iter().last() {
            Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
            _ => anyhow::bail!("Expect peer multiaddr to contain peer ID."),
        };
        swarm
            .behaviour_mut()
            .kad
            .add_address(&peer_id, addr.clone().with(Protocol::P2p(peer_id.into())));
        swarm
            .behaviour_mut()
            .kad
            .get_closest_peers(*node_identity.peer_id());
    }

    tokio::task::spawn(async move {
        let state = network_state.clone();
        let blockchain = blockchain.clone();
        let request_handler = request_handler.clone();
        loop {
            tokio::select! {
            msg = node_to_p2p.recv() => {
                    if let Some(msg) = msg {
                        if let Some(peer_id) = msg.peer_id {
                            let res = handle_send_message_to_peer(&mut swarm, peer_id, msg.message).await;
                            if let Err(error) = res {
                                debug!(error = ?error, "Error handling sending message to peer");
                            }
                        }else {
                            handle_publish_message(msg.message, &mut swarm).await;
                        }
                    }
                }
            event = swarm.select_next_some() => {
                    let res =  handle_swam_event(event, &mut swarm, &state, &blockchain, &request_handler).await;
                    if let Err(error) = res {
                        debug!(error = ?error, "Error handling swarm event");
                    }
                }
            }
        }
    });
    Ok(())
}

async fn handle_publish_message(msg: Msg, swarm: &mut Swarm<ChainNetworkBehavior>) {
    let msg: PeerMessage = msg.into();
    match msg.encode() {
        Ok(encoded_msg) => {
            let network_topic = swarm.behaviour_mut().topic.clone();
            let res = swarm
                .behaviour_mut()
                .gossipsub
                .publish(network_topic, encoded_msg);
            match res {
                Ok(message_id) => {
                    debug!("Publish gossip message_id {}", message_id);
                }
                Err(error) => {
                    debug!(error = ?error, "Publish gossip message");
                }
            }
        }
        Err(error) => {
            debug!(error = ?error, "Publish gossip message");
        }
    }
}

async fn handle_swam_event<T: std::fmt::Debug>(
    event: SwarmEvent<OutEvent, T>,
    swarm: &mut Swarm<ChainNetworkBehavior>,
    network_state: &Arc<NetworkState>,
    blockchain: &Arc<dyn Blockchain>,
    request_handler: &RequestHandler,
) -> Result<()> {
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
            message,
            ..
        })) => {
            if let Some(peer_message) = PeerMessage::decode(&message.data)?.msg {
                if let Msg::CurrentHead(msg) = &peer_message {
                    network_state
                        .update_peer_current_head(&propagation_source, *msg.block_header()?)?;
                }
                swarm.behaviour_mut().p2p_to_node.send(peer_message)?;
            }
        }

        SwarmEvent::Behaviour(OutEvent::Gossipsub(GossipsubEvent::Unsubscribed {
            peer_id,
            topic,
        })) => {
            if topic == swarm.behaviour_mut().topic.hash()
                && swarm.disconnect_peer_id(peer_id).is_err()
            {
                debug!(peer_id = ?peer_id, "Failed to disconnect peer");
            }
        }

        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
            for (peer, addr) in list {
                info!(peer = ?addr, "New Peer discovered");
                if !swarm.is_connected(&peer) {
                    swarm.dial(addr.clone()).unwrap();
                }
            }
        }
        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
            for (peer, addr) in list {
                warn!(peer = ?addr, "Peer expired");
                if !swarm.behaviour_mut().mdns.has_node(&peer)
                    && swarm.disconnect_peer_id(peer).is_err()
                {
                    debug!(peer_id = ?peer, "Failed to disconnect peer")
                }
            }
        }

        SwarmEvent::Behaviour(OutEvent::Kademlia(KademliaEvent::OutboundQueryCompleted {
            result: QueryResult::GetClosestPeers(Ok(results)),
            ..
        })) => {
            info!(list = ?results.peers,"Found Peers");
        }

        SwarmEvent::Behaviour(OutEvent::Kademlia(KademliaEvent::RoutingUpdated {
            peer,
            is_new_peer,
            addresses,
            ..
        })) => {
            if is_new_peer {
                info!(peer = ?peer,"New Peer");
                swarm.behaviour_mut().kad.get_closest_peers(peer);
                for address in addresses.iter() {
                    info!(address = ?address,"Dialing new peer");
                    swarm.dial(address.clone())?
                }
            }
        }

        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message {
            peer,
            message,
        })) => match message {
            RequestResponseMessage::Request {
                request, channel, ..
            } => match &request {
                Msg::Ack(_) => {
                    let chain_network = swarm.behaviour_mut();
                    let _ = chain_network.requestresponse.send_response(
                        channel,
                        Msg::ReAck(ReAckMessage::new(
                            chain_network.node,
                            // TODO: Refactor can cause error crashes
                            blockchain.current_header().unwrap().unwrap().raw,
                        )),
                    );
                }
                message => {
                    request_handler
                        .handle(&peer, message)
                        .map_err(|e| {
                            debug!(error = ?e, "failed to handle request");
                            message.clone()
                        })
                        .and_then(|resp| {
                            let chain_network = swarm.behaviour_mut();
                            if let Some(resp) = resp {
                                return chain_network.requestresponse.send_response(channel, resp);
                            }
                            Ok(())
                        })
                        .map_err(|msg| anyhow!("failed to respond to peer message {:?}", msg))?;
                }
            },

            RequestResponseMessage::Response {
                request_id,
                response,
            } => match &response {
                Msg::ReAck(msg) => {
                    let peers = swarm.behaviour().state.peer_list();
                    match peers.promote_peer(
                        &peer,
                        request_id,
                        msg.node_info()?,
                        swarm.behaviour().pow_target,
                    ) {
                        Ok((peer, address)) => {
                            let chain_network = swarm.behaviour_mut();
                            chain_network.gossipsub.add_explicit_peer(&peer);
                            chain_network.kad.add_address(&peer, address.clone());
                            network_state.update_peer_current_head(&peer, msg.current_header()?)?;

                            info!(peer = ?peer, peer_node_info = ?msg.node_info, stats = ?peers.stats(),"Connected to new peer");
                            network_state.handle_new_peer_connected(&peer)?
                        }
                        Err(error) => {
                            warn!(peer = ?&peer, error = ?error,"Failed to promote peer");
                            if swarm.disconnect_peer_id(peer).is_err() {
                                debug!(peer_id = ?peer, "Failed to disconnect peer")
                            }
                        }
                    }
                }
                _ => {
                    swarm.behaviour_mut().p2p_to_node.send(response)?;
                }
            },
        },

        SwarmEvent::Behaviour(OutEvent::RequestResponse(
            RequestResponseEvent::OutboundFailure { .. },
        )) => {}

        SwarmEvent::ConnectionEstablished {
            peer_id,
            endpoint: ConnectedPoint::Dialer { address },
            ..
        } => {
            let chain_network = swarm.behaviour_mut();
            let peers = chain_network.state.peer_list();
            if !peers.is_peer_connected(&peer_id) {
                let request_id = swarm
                    .behaviour_mut()
                    .requestresponse
                    .send_request(&peer_id, Msg::Ack(AckMessage::new()));
                peers.add_potential_peer(peer_id, request_id);
                peers.set_peer_address(peer_id, address.clone());
            }
            info!(peer = ?address,"Connection established");
        }
        SwarmEvent::ConnectionClosed {
            endpoint: ConnectedPoint::Dialer { address },
            cause,
            peer_id,
            ..
        } => {
            if let Some(cause) = cause {
                swarm
                    .behaviour_mut()
                    .gossipsub
                    .remove_explicit_peer(&peer_id);
                swarm.behaviour_mut().kad.remove_peer(&peer_id);
                match swarm.behaviour_mut().state.remove_peer(&peer_id) {
                    Ok(_) => {}
                    Err(error) => {
                        debug!(error = ?error, "Error removing peer");
                    }
                };
                warn!(peer = ?peer_id, address = ?address, cause = ?cause, "Connection closed");
            } else {
                debug!(peer = ?peer_id, address = ?address, cause = "Unknown", "Connection closed")
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
struct ChainNetworkBehavior {
    gossipsub: Gossipsub,
    mdns: Mdns,
    kad: Kademlia<MemoryStore>,
    requestresponse: RequestResponse<ChainP2pExchangeCodec>,
    #[behaviour(ignore)]
    p2p_to_node: UnboundedSender<Msg>,
    #[behaviour(ignore)]
    topic: Sha256Topic,
    #[behaviour(ignore)]
    node: PeerNode,
    #[behaviour(ignore)]
    state: Arc<NetworkState>,
    #[behaviour(ignore)]
    pow_target: U256,
}

#[derive(Debug)]
enum OutEvent {
    Gossipsub(GossipsubEvent),
    RequestResponse(RequestResponseEvent<Msg, Msg>),
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

impl From<RequestResponseEvent<Msg, Msg>> for OutEvent {
    fn from(v: RequestResponseEvent<Msg, Msg>) -> Self {
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
    type Request = Msg;
    type Response = Msg;

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
        PeerMessage::decode(&data)
            .and_then(|message| message.msg.ok_or_else(|| anyhow::anyhow!("no data")))
            .map_err(|_| std::io::ErrorKind::Unsupported.into())
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
        PeerMessage::decode(&data)
            .and_then(|message| message.msg.ok_or_else(|| anyhow::anyhow!("no data")))
            .map_err(|_| std::io::ErrorKind::Unsupported.into())
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
        let encoded = PeerMessage::new(req)
            .encode()
            .map_err(|_| std::io::Error::from(ErrorKind::UnexpectedEof))?;
        write_length_prefixed(io, encoded).await?;
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
        let encoded = PeerMessage::new(res)
            .encode()
            .map_err(|_| std::io::Error::from(ErrorKind::UnexpectedEof))?;
        write_length_prefixed(io, encoded).await?;
        io.close().await?;
        Ok(())
    }
}
