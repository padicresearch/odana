use anyhow::{Error, Result};
use async_bincode::AsyncBincodeReader;
use async_trait::async_trait;
use colored::Colorize;
use hex::ToHex;
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use libp2p::core::{identity, ProtocolName};
use libp2p::core::connection::ConnectionError;
use libp2p::core::either::EitherError;
use libp2p::core::identity::ed25519;
use libp2p::core::transport::upgrade::Version;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::floodsub::{Floodsub, FloodsubEvent};
use libp2p::futures::{AsyncRead, AsyncWrite, AsyncWriteExt, StreamExt};
use libp2p::gossipsub::{
    Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity, Sha256Topic, Topic,
    ValidationMode,
};
use libp2p::identity::Keypair;
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::multiaddr::Protocol;
use libp2p::NetworkBehaviour;
use libp2p::noise::{AuthenticKeypair, NoiseConfig, X25519Spec};
use libp2p::request_response::{RequestResponse, RequestResponseCodec, RequestResponseEvent, RequestResponseMessage};
use libp2p::swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent};
use libp2p::swarm::protocols_handler::NodeHandlerWrapperError;
use libp2p::tcp::TokioTcpConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use codec::{Codec, Decoder, Encoder};
use codec::impl_codec;
use tracing::{info, warn};
use types::block::{Block, BlockHeader};
use types::Hash;
use types::tx::Transaction;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CurrentHeadMessage {
    pub block_header: BlockHeader,
}

impl CurrentHeadMessage {
    pub fn new(block_header: BlockHeader) -> Self {
        Self { block_header }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BroadcastTransactionMessage {
    tx: Transaction,
}

impl BroadcastTransactionMessage {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    pub fn tx(self) -> Transaction {
        self.tx
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BroadcastBlockMessage {
    block: Block,
}
impl BroadcastBlockMessage {
    pub fn new(block: Block) -> Self {
        Self { block }
    }

    pub fn block(self) -> Block {
        self.block
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetCurrentHeadMessage {
    pub sender: String,
}

impl GetCurrentHeadMessage {
    pub fn new(sender: String) -> Self {
        Self { sender }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetBlockHeaderMessage {
    pub sender: String,
    pub block_hashes: Vec<Hash>,
}

impl GetBlockHeaderMessage {
    pub fn new(sender: String, block_hashes: Vec<Hash>) -> Self {
        Self {
            sender,
            block_hashes,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BlockTransactionsMessage {
    pub recipient: String,
    pub txs: Vec<Transaction>,
}

impl BlockTransactionsMessage {
    pub fn new(recipient: String, txs: Vec<Transaction>) -> Self {
        Self { recipient, txs }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BlockHeaderMessage {
    pub recipient: String,
    pub block_headers: Vec<BlockHeader>,
}

impl BlockHeaderMessage {
    pub fn new(recipient: PeerId, block_headers: Vec<BlockHeader>) -> Self {
        Self {
            recipient: recipient.to_string(),
            block_headers,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetBlockTransactionsMessage {
    pub sender: String,
    pub tx_ids: Vec<Hash>,
}

impl GetBlockTransactionsMessage {
    pub fn new(sender: PeerId, tx_ids: Vec<Hash>) -> Self {
        Self {
            sender: sender.to_string(),
            tx_ids,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum PeerMessage {
    GetCurrentHead(GetCurrentHeadMessage),
    CurrentHead(CurrentHeadMessage),
    GetBlockHeader(GetBlockHeaderMessage),
    BlockHeader(BlockHeaderMessage),
    GetBlock(Block),
    Block(Block),
    BroadcastTransaction(BroadcastTransactionMessage),
    BroadcastBlock(BroadcastBlockMessage),
}

impl_codec!(PeerMessage);

pub struct Peer {}

pub struct NodeIdentity {
    pub_key: libp2p::identity::ed25519::PublicKey,
    secret_key: libp2p::identity::ed25519::SecretKey,
    peer_id: PeerId,
}

impl NodeIdentity {
    pub fn new(_pow: [u8; 32]) -> Self {
        let keys = libp2p::identity::ed25519::Keypair::generate();

        let pub_key = keys.public();
        let secret_key = keys.secret();

        let peer_id =
            PeerId::from_public_key(&libp2p::identity::PublicKey::Ed25519(pub_key.clone()));

        Self {
            pub_key,
            secret_key,
            peer_id,
        }
    }

    pub fn generate() -> Self {
        let keys = libp2p::identity::ed25519::Keypair::generate();

        let pub_key = keys.public();
        let secret_key = keys.secret();

        let peer_id =
            PeerId::from_public_key(&libp2p::identity::PublicKey::Ed25519(pub_key.clone()));

        Self {
            pub_key,
            secret_key,
            peer_id,
        }
    }

    pub fn identity_keys(&self) -> libp2p::identity::Keypair {
        let keys = libp2p::identity::Keypair::Ed25519(libp2p::identity::ed25519::Keypair::from(
            self.secret_key.clone(),
        ));
        keys
    }
}

async fn config_network(
    node_identity: NodeIdentity,
    p2p_to_node: UnboundedSender<PeerMessage>,
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
    let max_transmit_size = 500;
    let config = GossipsubConfigBuilder::default()
        .max_transmit_size(max_transmit_size)
        .protocol_id_prefix("tuchain")
        .validation_mode(ValidationMode::Permissive)
        .build()
        .unwrap();
    let mut behaviour = ChainNetworkBehavior {
        gossipsub: Gossipsub::new(
            MessageAuthenticity::Author(node_identity.peer_id.clone()),
            config,
        )
            .expect("Failed to create Gossip sub network"),
        mdns,
        p2p_to_node,
        topic: network_topic.clone(),
    };

    behaviour.gossipsub.subscribe(&network_topic);

    let swarm = SwarmBuilder::new(transport, behaviour, node_identity.peer_id)
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
) -> Result<()> {
    let mut swarm = config_network(node_identity, p2p_to_node).await?;

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9020".parse()?)
        .expect("Error connecting to p2p");

    if let Some(to_dial) = peer_arg {
        let addr: Multiaddr = to_dial.parse()?;
        let peer_id = match addr.iter().last() {
            Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
            _ => anyhow::bail!("Expect peer multiaddr to contain peer ID."),
        };
        swarm.dial(addr.with(Protocol::P2p(peer_id.into())))?;
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
                "connection listening on {}",
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
        SwarmEvent::Behaviour(OutEvent::RequestResponse(RequestResponseEvent::Message { peer, message })) => {
            match message {
                RequestResponseMessage::Request { request_id, request, .. } => {},

                RequestResponseMessage::Response { request_id, response } => {}
            }
        }

        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            info!(peer = ?endpoint.get_remote_address(),"Connection established");
        }
        SwarmEvent::ConnectionClosed {
            endpoint, cause, ..
        } => {
            if let Some(cause) = cause {
                swarm.dial(endpoint.get_remote_address().clone()).unwrap();
                warn!(peer = ?endpoint.get_remote_address(), cause = ?cause, "Connection closed");
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
    #[behaviour(ignore)]
    p2p_to_node: UnboundedSender<PeerMessage>,
    #[behaviour(ignore)]
    topic: Sha256Topic,
}

#[derive(Debug)]
enum OutEvent {
    Gossipsub(GossipsubEvent),
    RequestResponse(RequestResponseEvent<PeerMessage, PeerMessage>),
    Mdns(MdnsEvent),
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

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request> where T: AsyncRead + Unpin + Send {
        let data = read_length_prefixed(io, 1_000_000).await?;
        if data.is_empty() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let message: Result<PeerMessage> = PeerMessage::decode(&data);
        let message = match message {
            Ok(message) => {
                message
            }
            Err(_) => {
                return Err(std::io::ErrorKind::Unsupported.into())
            }
        };
        Ok(message)
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response> where T: AsyncRead + Unpin + Send {
        let data = read_length_prefixed(io, 1_000_000).await?;
        if data.is_empty() {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        let message: Result<PeerMessage> = PeerMessage::decode(&data);
        let message = match message {
            Ok(message) => {
                message
            }
            Err(_) => {
                return Err(std::io::ErrorKind::Unsupported.into())
            }
        };
        Ok(message)
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, req: Self::Request) -> std::io::Result<()> where T: AsyncWrite + Unpin + Send {
        write_length_prefixed(io, req.encode().unwrap()).await?;
        io.close().await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, res: Self::Response) -> std::io::Result<()> where T: AsyncWrite + Unpin + Send {
        write_length_prefixed(io, res.encode().unwrap()).await?;
        io.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::account::create_account;

    #[test]
    fn account_to_node_id() {}
}
