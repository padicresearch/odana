use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use dashmap::DashMap;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::mpsc::UnboundedSender;

use types::block::BlockHeader;
use types::events::LocalEventMessage;

#[derive(Debug, Clone)]
pub struct PeerList {
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
            addrs: Default::default(),
        }
    }

    pub fn set_peer_address(&self, peer: PeerId, addr: Multiaddr) {
        let peer_id = Arc::new(peer);
        self.addrs.insert(peer_id, addr);
    }

    pub fn remove_peer(&self, peer: &PeerId) {
        self.addrs.remove(peer);
    }

    pub fn stats(&self) -> (usize, usize) {
        (0, self.addrs.len())
    }

    pub fn get_peer(&self, peer: &PeerId) -> Option<Arc<PeerId>> {
        self.addrs.get(peer).map(|r| r.key().clone())
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
        let peer_state = self.peer_state.clone();
        let mut peer_state = peer_state.write().unwrap();
        let peer = peer_state
            .get_key_value(peer_id)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| Arc::new(*peer_id));
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
            if peer_state.insert(peer.clone(), head).is_none() {
                self.handle_new_peer_connected(peer_id)?;
            };
            self.sender
                .send(LocalEventMessage::NetworkHighestHeadChanged {
                    peer_id: peer.to_string(),
                    tip: Some(head),
                })?;
        }
        Ok(())
    }

    pub fn handle_new_peer_connected(&self, peer_id: &PeerId) -> Result<()> {
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
    use crate::identity::NodeIdentity;

    #[test]
    fn check_pow() {
        let node_identity = NodeIdentity::generate();
        println!("Stramp {:#?}", node_identity.to_p2p_node());
    }
}
