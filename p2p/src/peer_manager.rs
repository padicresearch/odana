use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use anyhow::bail;
use anyhow::Result;
use dashmap::DashMap;
use libp2p::request_response::RequestId;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::Compact;
use types::block::BlockHeader;
use types::events::LocalEventMessage;

use crate::identity::PeerNode;

#[derive(Debug, Clone)]
pub struct PeerList {
    potential_peers: DashMap<Arc<PeerId>, RequestId>,
    connected_peers: DashMap<Arc<PeerId>, PeerNode>,
    addrs: DashMap<Arc<PeerId>, Multiaddr>,
}

impl PeerList {
    pub fn new() -> Self {
        Self {
            potential_peers: Default::default(),
            connected_peers: Default::default(),
            addrs: Default::default(),
        }
    }

    pub fn add_potential_peer(&self, peer: PeerId, request_id: RequestId) {
        let peer_id = Arc::new(peer);
        self.potential_peers.insert(peer_id.clone(), request_id);
    }

    pub fn set_peer_address(&self, peer: PeerId, addr: Multiaddr) {
        let peer_id = Arc::new(peer);
        self.addrs.insert(peer_id, addr);
    }

    pub fn promote_peer(
        &self,
        peer: &PeerId,
        request_id: RequestId,
        node: PeerNode,
        pow_target: Compact,
    ) -> Result<()> {
        if self.connected_peers.contains_key(peer) {
            return Ok(());
        }
        match self.potential_peers.remove(peer) {
            None => {
                bail!("No potential peer")
            }
            Some((peer, id)) => {
                if id != request_id {
                    println!("Request id mismatch; excepted {}, found {}", id, request_id)
                }
                match node.peer_id() {
                    Ok(derived_peer_id) => {
                        if derived_peer_id != *peer {
                            bail!("Invalid PeerId mismatch by node {}", peer)
                        }
                    }
                    Err(_) => {
                        bail!("Invalid PeerId mismatch by node {}", peer);
                    }
                }

                if !crypto::is_valid_proof_of_work_hash(pow_target, &node.pow()) {
                    bail!("Invalid Proof of work by node {}", peer)
                }
                self.connected_peers.insert(peer, node);
                Ok(())
            }
        }
    }

    pub fn remove_peer(&self, peer: &PeerId) {
        self.potential_peers.remove(peer);
        self.connected_peers.remove(peer);
        self.addrs.remove(peer);
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.potential_peers.len(), self.connected_peers.len())
    }

    pub fn get_peer(&self, peer: &PeerId) -> Option<Arc<PeerId>> {
        self.connected_peers.get(peer).map(|r| r.key().clone())
    }

    pub fn potential_peers<'a>(&'a self) -> Box<dyn Iterator<Item = Arc<PeerId>> + 'a> {
        return Box::new(self.potential_peers.iter().map(|r| r.key().clone()));
    }

    pub fn connected_peers<'a>(&'a self) -> Box<dyn Iterator<Item = Arc<PeerId>> + 'a> {
        return Box::new(self.connected_peers.iter().map(|r| r.key().clone()));
    }

    pub fn is_peer_connected(&self, peer: &PeerId) -> bool {
        self.connected_peers.contains_key(peer)
    }

    pub fn peers_addrs(&self) -> BTreeSet<Multiaddr> {
        self.addrs.iter().map(|peer| peer.value().clone()).collect()
    }

    pub fn random_connected_peer(&self) -> &PeerId {
        todo!()
    }
}

pub struct NetworkState {
    peer_list: Arc<PeerList>,
    peer_state: DashMap<Arc<PeerId>, BlockHeader>,
    highest_know_head: RwLock<Option<Arc<PeerId>>>,
    sender: UnboundedSender<LocalEventMessage>,
}

impl NetworkState {
    pub fn new(peer_list: Arc<PeerList>, sender: UnboundedSender<LocalEventMessage>) -> Self {
        Self {
            peer_list,
            peer_state: Default::default(),
            highest_know_head: RwLock::default(),
            sender,
        }
    }

    pub fn update_peer_current_head(&self, peer_id: &PeerId, head: BlockHeader) -> Result<()> {
        anyhow::ensure!(
            self.peer_list.is_peer_connected(peer_id),
            "Peer is not connected"
        );
        let peer = self.peer_list.get_peer(peer_id).unwrap();
        self.sender
            .send(LocalEventMessage::NetworkNewPeerConnection {
                stats: self.peer_list.stats(),
            });
        let mut highest_know_head = self.highest_know_head.write().unwrap();
        if highest_know_head.is_none() {
            let mut new_highest = Some(peer.clone());
            std::mem::swap(&mut *highest_know_head, &mut new_highest);
            self.peer_state.insert(peer.clone(), head.clone());
            self.sender
                .send(LocalEventMessage::NetworkHighestHeadChanged {
                    peer_id: peer.to_string(),
                    current_head: head,
                });
        } else {
            let mut new_highest = Some(peer.clone());
            let current_highest_peer_id = highest_know_head.as_ref().cloned().unwrap();
            let current_highest_block_header = self
                .peer_state
                .get(&current_highest_peer_id).unwrap();
            if head.level > current_highest_block_header.level {
                std::mem::swap(&mut *highest_know_head, &mut new_highest);
                self.peer_state.insert(peer.clone(), head.clone());
                self.sender
                    .send(LocalEventMessage::NetworkHighestHeadChanged {
                        peer_id: peer.to_string(),
                        current_head: head,
                    });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use primitive_types::U256;

    use crate::identity::NodeIdentity;
    use crate::p2p::NodeIdentity;

    pub const NODE_POW_TARGET: U256 = U256([
        0x0000000000000000u64,
        0x0000000000000000u64,
        0x0000000000000000u64,
        0x00000fffff000000u64,
    ]);

    #[test]
    fn check_pow() {
        let node_iden = NodeIdentity::generate(NODE_POW_TARGET.into());
        println!("Stramp {:#?}", node_iden.to_p2p_node());
    }
}
