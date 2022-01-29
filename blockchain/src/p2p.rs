use anyhow::{Error, Result};
use hex::ToHex;
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use libp2p::core::either::EitherError;
use libp2p::core::identity;
use libp2p::core::identity::ed25519;
use libp2p::core::transport::upgrade::Version;
use libp2p::floodsub::{Floodsub, FloodsubEvent};
use libp2p::futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::multiaddr::Protocol;
use libp2p::NetworkBehaviour;
use libp2p::noise::{AuthenticKeypair, NoiseConfig, X25519Spec};
use libp2p::request_response::RequestResponse;
use libp2p::swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent};
use libp2p::tcp::TokioTcpConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use codec::{Codec, Decoder, Encoder};
use codec::impl_codec;
use tracing::info;
use types::block::{Block, BlockHeader};
use types::Hash;
use types::tx::Transaction;

#[derive(Serialize, Deserialize, Debug)]
pub struct CurrentHeadMessage {
    pub block_header: BlockHeader,
}

impl CurrentHeadMessage {
    pub fn new(block_header: BlockHeader) -> Self {
        Self { block_header }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCurrentHeadMessage {
    pub sender: String,
}

impl GetCurrentHeadMessage {
    pub fn new(sender: String) -> Self {
        Self { sender }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockTransactionsMessage {
    pub recipient: String,
    pub txs: Vec<Transaction>,
}

impl BlockTransactionsMessage {
    pub fn new(recipient: String, txs: Vec<Transaction>) -> Self {
        Self { recipient, txs }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

    let network_topic = libp2p::floodsub::Topic::new("testnet");

    let mdns = Mdns::new(Default::default())
        .await
        .expect("Cannot create mdns");
    let mut behaviour = ChainNetworkBehavior {
        floodsub: Floodsub::new(node_identity.peer_id.clone()),
        mdns,
        p2p_to_node,
    };

    behaviour.floodsub.subscribe(network_topic);

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
    peer_arg: Option<String>
) -> Result<()> {
    let mut swarm = config_network(node_identity, p2p_to_node).await?;
    if let Some(to_dial) = peer_arg {
        let addr: Multiaddr = to_dial.parse()?;
        let peer_id = match addr.iter().last() {
            Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
            _ => anyhow::bail!("Expect peer multiaddr to contain peer ID."),
        };
        swarm.dial(addr.with(Protocol::P2p(peer_id.into())))?;
        swarm
            .behaviour_mut()
            .floodsub
            .add_node_to_partial_view(peer_id);
        println!("Dialed {:?}", to_dial)
    }
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/9020".parse()?)
        .expect("Error connecting to p2p");


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
            info!("sending flood message {:?}", msg);
            let network_topic = libp2p::floodsub::Topic::new("testnet");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(network_topic, encoded_msg);
        } else {
            println!("Failed to encode message {:?}", msg)
        }
    }
}

async fn handle_swam_event<T>(
    event: SwarmEvent<OutEvent, T>,
    swarm: &mut Swarm<ChainNetworkBehavior>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            let local_peer_id = *swarm.local_peer_id();
            println!("Listening on {}", address.with(Protocol::P2p(local_peer_id.into())));
        }
        SwarmEvent::Behaviour(OutEvent::Floodsub(FloodsubEvent::Message(message))) => {
            info!("new flood message {:?}", message);
            if let Ok(peer_message) = PeerMessage::decode(&message.data) {
                swarm.behaviour_mut().p2p_to_node.send(peer_message);
            }
        }
        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Discovered(list))) => {
            for (peer, addr) in list {
                info!("new peer discovered {:?}", addr);
                swarm
                    .behaviour_mut()
                    .floodsub
                    .add_node_to_partial_view(peer);
            }
        }
        SwarmEvent::Behaviour(OutEvent::Mdns(MdnsEvent::Expired(list))) => {
            for (peer, _) in list {
                if !swarm.behaviour_mut().mdns.has_node(&peer) {
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .remove_node_from_partial_view(&peer);
                }
            }
        }

        SwarmEvent::ConnectionEstablished { peer_id, .. } => {}
        _ => {}
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
struct ChainNetworkBehavior {
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    p2p_to_node: UnboundedSender<PeerMessage>,
}

#[derive(Debug)]
enum OutEvent {
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
}

impl From<MdnsEvent> for OutEvent {
    fn from(v: MdnsEvent) -> Self {
        Self::Mdns(v)
    }
}

impl From<FloodsubEvent> for OutEvent {
    fn from(v: FloodsubEvent) -> Self {
        Self::Floodsub(v)
    }
}
enum P2PEvent {}

#[cfg(test)]
mod tests {
    use crate::account::create_account;

    #[test]
    fn account_to_node_id() {}
}
