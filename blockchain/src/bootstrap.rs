use crate::mempool::MempoolSnapsot;
use libp2p::PeerId;
use std::collections::{HashMap, HashSet};
use types::block::BlockHeader;

struct Bootstrapper {}
/*
pub block_header: BlockHeader,
    pub mempool: MempoolSnapsot,
 */
#[derive(Debug, Clone)]
struct PeerState {
    current_head: BlockHeader,
    mempool: MempoolSnapsot,
}

struct BootstrapState {
    pending_peers: HashMap<PeerId, PeerState>,
    connected_peers: HashMap<PeerId, PeerState>,
}
