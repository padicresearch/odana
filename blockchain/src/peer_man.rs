use anyhow::Result;
use dashmap::{DashMap, DashSet};
use libp2p::PeerId;

use crypto::SHA256;
use primitive_types::{H160, H256, H448, U128, U192};

#[derive(Debug, Clone)]
pub struct PeerManager {
    potential_peers: DashSet<PeerId>,
    connected_peers: DashSet<PeerId>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            potential_peers: Default::default(),
            connected_peers: Default::default(),
        }
    }

    pub fn add_potential_peer(&self, peer: PeerId) {
        todo!()
    }

    pub fn promote_peer(&self, peer: &PeerId) -> bool {
        todo!()
    }

    pub fn remove_peer(&self, peer: &PeerId) -> Option<PeerId> {
        todo!()
    }

    pub fn potential_peers(&self) -> Vec<PeerId> {
        todo!()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        todo!()
    }

    pub fn random_connected_peer(&self) -> &PeerId {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crypto::{generate_pow_from_pub_key, is_valid_proof_of_work_hash, SHA256};
    use primitive_types::{Compact, H256, H448, U192, U256};
    use types::account::get_address_from_pub_key;

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
