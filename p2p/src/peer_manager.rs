use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

use anyhow::bail;
use anyhow::Result;
use dashmap::DashMap;
use libp2p::request_response::RequestId;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::U256;
use tracing::warn;
use types::block::BlockHeader;
use types::events::LocalEventMessage;

use crate::identity::PeerNode;

#[derive(Debug, Clone)]
pub struct PeerList {
    potential_peers: DashMap<Arc<PeerId>, RequestId>,
    connected_peers: DashMap<Arc<PeerId>, PeerNode>,
    addrs: DashMap<Arc<PeerId>, Multiaddr>,
}

impl Default for PeerList {
    fn default() -> Self {
        PeerList::new()
    }
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
        self.potential_peers.insert(peer_id, request_id);
    }

    pub fn set_peer_address(&self, peer: PeerId, addr: Multiaddr) {
        let peer_id = Arc::new(peer);
        self.addrs.insert(peer_id, addr);
    }

    // TODO: use error enums
    pub fn promote_peer(
        &self,
        peer: &PeerId,
        request_id: RequestId,
        node: PeerNode,
        pow_target: U256,
    ) -> Result<(Arc<PeerId>, Multiaddr)> {
        if self.connected_peers.contains_key(peer) {
            bail!("peer already connected")
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
                let addr = self
                    .addrs
                    .get(&peer)
                    .map(|t| t.value().clone())
                    .ok_or_else(|| anyhow::anyhow!("peer address not found"))?;
                self.connected_peers.insert(peer.clone(), node);

                Ok((peer, addr))
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
    peer_state: Arc<RwLock<HashMap<Arc<PeerId>, BlockHeader>>>,
    highest_know_head: RwLock<Option<Arc<PeerId>>>,
    sender: UnboundedSender<LocalEventMessage>,
}

impl NetworkState {
    pub fn new(sender: UnboundedSender<LocalEventMessage>) -> Self {
        Self {
            peer_list: Arc::new(PeerList::new()),
            peer_state: Default::default(),
            highest_know_head: RwLock::default(),
            sender,
        }
    }

    pub fn peer_list(&self) -> Arc<PeerList> {
        self.peer_list.clone()
    }

    pub fn update_peer_current_head(&self, peer_id: &PeerId, head: BlockHeader) -> Result<()> {
        if !self.peer_list.is_peer_connected(peer_id) {
            warn!(peer = ?peer_id, "Update Peer Head Error: Peer not connected");
            bail!("Peer is not connected")
        }
        let peer_state = self.peer_state.clone();
        let mut peer_state = peer_state.write().unwrap();
        let peer = self.peer_list.get_peer(peer_id).unwrap();
        let mut highest_know_head = self.highest_know_head.write().unwrap();
        if let Some(highest_know_head) = highest_know_head.as_mut() {
            if *highest_know_head != peer {
                let current_highest_peer_id = highest_know_head.clone();
                let current_highest_block_header = peer_state
                    .get(&current_highest_peer_id)
                    .ok_or_else(|| anyhow::anyhow!("Current highest peer not found"))?;
                let new_highest = peer.clone();
                if head.level() > current_highest_block_header.level() {
                    *highest_know_head = new_highest;
                    peer_state.insert(peer.clone(), head);
                }
            }

            self.sender
                .send(LocalEventMessage::NetworkHighestHeadChanged {
                    peer_id: peer.to_string(),
                    tip: Some(head),
                })?;
        } else {
            let new_highest = Some(peer.clone());
            *highest_know_head = new_highest;
            peer_state.insert(peer.clone(), head);
            self.sender
                .send(LocalEventMessage::NetworkHighestHeadChanged {
                    peer_id: peer.to_string(),
                    tip: Some(head),
                })?;
        }
        Ok(())
    }

    pub fn handle_new_peer_connected(&self, peer_id: &PeerId) -> Result<()> {
        anyhow::ensure!(
            self.peer_list.is_peer_connected(peer_id),
            "Peer is not connected"
        );

        self.sender
            .send(LocalEventMessage::NetworkNewPeerConnection {
                stats: self.peer_list.stats(),
                peer_id: peer_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn get_peer_state(&self, peer_id: &PeerId) -> Option<BlockHeader> {
        let peer_state = self.peer_state.read().unwrap();
        return peer_state.get(peer_id).copied();
    }

    pub fn remove_peer(&self, peer_id: &PeerId) -> Result<()> {
        {
            let mut highest_know_head = self.highest_know_head.write().unwrap();
            let peer_state = self.peer_state.clone();
            if highest_know_head
                .as_ref()
                .map(|highest_know_head| highest_know_head.as_ref().eq(peer_id))
                .unwrap_or(false)
            {
                *highest_know_head = None;
            }
            let mut peer_state = peer_state.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            peer_state.remove(peer_id);
            self.peer_list.remove_peer(peer_id);
        }

        self.sender
            .send(LocalEventMessage::NetworkHighestHeadChanged {
                peer_id: peer_id.to_string(),
                tip: self.network_head(),
            })
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn highest_peer(&self) -> Option<String> {
        let highest_know_head = self.highest_know_head.read().unwrap();
        highest_know_head
            .as_ref()
            .map(|peer_id| peer_id.to_string())
    }

    pub fn highest_peer_raw(&self) -> Option<Arc<PeerId>> {
        let highest_know_head = self.highest_know_head.read().unwrap();
        highest_know_head.clone()
    }

    pub fn network_head(&self) -> Option<BlockHeader> {
        let highest_peer = self.highest_peer_raw();
        match highest_peer {
            None => None,
            Some(peer_id) => {
                let peer_state = self.peer_state.clone();
                let peer_state = peer_state.read().unwrap();
                peer_state.get(&peer_id).cloned()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use primitive_types::U256;

    use crate::identity::NodeIdentity;

    pub const NODE_POW_TARGET: U256 = U256([
        0x0000000000000000u64,
        0x0000000000000000u64,
        0x0000000000000000u64,
        0x00000fffff000000u64,
    ]);

    #[test]
    fn check_pow() {
        let node_identity = NodeIdentity::generate(NODE_POW_TARGET);
        println!("Stramp {:#?}", node_identity.to_p2p_node());
    }
}
