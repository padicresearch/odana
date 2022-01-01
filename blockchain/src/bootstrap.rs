use std::collections::{HashSet, HashMap};
use libp2p::PeerId;
use crate::mempool::MempoolSnapsot;

struct Bootstrapper {

}
/*
pub block_header: BlockHeader,
    pub mempool: MempoolSnapsot,
 */
#[derive(Debug, Clone)]
struct PeerState {
    current_head :  BlockHeader,
    mempool : MempoolSnapsot
}

struct BootstrapState {
    pending_peers : HashMap<PeerId, PeerState>,
    connected_peers : HashMap<PeerId, PeerState>,

}