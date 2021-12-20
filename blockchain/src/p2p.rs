use libp2p::{PeerId, Transport, Swarm, Multiaddr};
use crate::account::Account;
use anyhow::{Result, Error};
use libp2p::tcp::TokioTcpConfig;
use libp2p::core::transport::upgrade::Version;
use libp2p::identity::Keypair;
use libp2p::noise::{X25519Spec, NoiseConfig, AuthenticKeypair};
use libp2p::floodsub::{Floodsub, FloodsubEvent};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::NetworkBehaviour;
use libp2p::swarm::{SwarmBuilder, NetworkBehaviourEventProcess};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::block::{BlockHeader, Block};
use common::TxHash;
use crate::mempool::MempoolSnapsot;
use storage::codec::{Encoder, Decoder, Codec};
use storage::codec::impl_codec;
use crate::transaction::Tx;

#[derive(Serialize, Deserialize)]
pub struct CurrentHeadMessage {
    block_header : BlockHeader,
    mempool : MempoolSnapsot,
    receiver : Option<String>
}



#[derive(Serialize, Deserialize)]
pub struct BroadcastTransactionMessage {
    tx : Tx,
}

impl BroadcastTransactionMessage {
    pub fn new(tx : Tx) -> Self {
        Self {
            tx
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BroadcastBlockMessage {
    block : Block,
}

impl BroadcastBlockMessage {
    pub fn new(block : Block) -> Self {
        Self {
            block
        }
    }
}




#[derive(Serialize, Deserialize)]
pub enum PeerMessage {
    GetCurrentHead,
    CurrentHead(CurrentHeadMessage),
    GetBlockHeader,
    BlockHeader,
    GetBlockTransactions,
    BlockTransactions,
    BroadcastTransaction(BroadcastTransactionMessage),
    BroadcastBlock(BroadcastBlockMessage),
}

impl_codec!(PeerMessage);


pub struct Peer {}

#[derive(Debug)]
pub struct NodeIdentity {
    keys: libp2p::identity::Keypair,
    peer_id: PeerId,
}

fn config_network(node_identity: NodeIdentity, p2p_to_node : UnboundedSender<PeerMessage>) -> Result<Swarm<ChainNetworkBehavior>> {
    let auth_keys : AuthenticKeypair<X25519Spec>= Keypair::<X25519Spec>::new()
        .into_authentic(&node_identity.keys)
        .expect("cannot create auth keys");

    let transport = TokioTcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(libp2p::mplex::MplexConfig::new())
        .boxed();

    let network_topic = libp2p::floodsub::Topic::new("testnet");

    let mut swarm = {
        let mdns = Mdns::new(Default::default());
        let mut behaviour = ChainNetworkBehavior {
            floodsub: Floodsub::new(node_identity.peer_id.clone()),
            mdns,
            p2p_to_node
        };

        behaviour.floodsub.subscribe(network_topic);

        SwarmBuilder::new(transport, behaviour, node_identity.peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    Ok(swarm)
}

pub fn start_p2p_server(node_identity: NodeIdentity, node_to_p2p: UnboundedReceiver<PeerMessage>, p2p_to_node : UnboundedSender<PeerMessage>) -> Result<()> {
    let mut swarm = config_network(node_identity, p2p_to_node)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let msg = tokio::select! {

    }


    Ok(())
}


#[derive(NetworkBehaviour)]
pub struct ChainNetworkBehavior {
    floodsub: Floodsub,
    mdns: Mdns,
    p2p_to_node : UnboundedSender<PeerMessage>
}


impl NetworkBehaviourEventProcess<MdnsEvent> for ChainNetworkBehavior {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_nodes) => {
                for (peer, addr) in discovered_nodes {
                    self.floodsub.add_node_to_partial_view(peer)
                }
            }
            MdnsEvent::Expired(expired_nodes) => {
                for (peer, addr) in expired_nodes {
                    self.floodsub.remove_node_from_partial_view(&peer)
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for ChainNetworkBehavior {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(msg) => {
                if let Ok(peer_message) = PeerMessage::decode(&msg.data) {
                    match peer_message {
                        PeerMessage::BroadcastTransaction(msg) => {
                            self.p2p_to_node.send()
                        }
                        PeerMessage::BroadcastBlock(msg) => {

                        }
                        _ => {}
                    }
                }
            }
            FloodsubEvent::Subscribed { .. } => {}
            FloodsubEvent::Unsubscribed { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::account::create_account;
    use crate::p2p::Node;

    #[test]
    fn account_to_node_id() {}
}