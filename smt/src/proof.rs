use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use serde::{Serialize, Deserialize};
use primitive_types::H256;
use crate::treehasher::TreeHasher;
use crate::utils::get_bits_at_from_msb;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Proof {
    pub side_nodes: Vec<H256>,
    pub non_membership_leaf_data: Option<Vec<u8>>,
    pub sibling_data: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
pub struct CompatProof {
    pub side_nodes: Vec<H256>,
    pub non_membership_leaf_data: Vec<u8>,
    pub bitmask: Vec<u8>,
    pub num_side_nodes: u32,
    pub sibling_data: Option<Vec<u8>>,
}

pub fn verify_proof(proof: Proof, root: H256, key: &[u8], value: &[u8]) -> bool {
    let (result, _) = verify_proof_with_updates(proof, root, key, value);
    return result;
}


fn verify_proof_with_updates(proof: Proof, root: H256, key: &[u8], value: &[u8]) -> (bool, Vec<Vec<Vec<u8>>>) {
    let th = TreeHasher::new();
    let path = th.path(key);
    let mut updates = Vec::new();

    let mut current_hash = H256::zero();
    let mut current_data = Vec::new();

    if value.is_empty() {
        if proof.non_membership_leaf_data.is_none() {
            current_hash = th.placeholder()
        } else if let Some(non_membership_leaf_data) = &proof.non_membership_leaf_data {
            let (acutal_path, value_hash) = th.parse_leaf(non_membership_leaf_data);
            if acutal_path.eq(path.as_bytes()) {
                return (false, Vec::new());
            }
            let (l, r) = th.digest_leaf(acutal_path, value_hash);
            current_hash = l;
            current_data = r;
            updates.push(vec![current_hash.as_bytes().to_vec(), current_data.to_vec()]);
        }
    } else {
        let value_hash = th.digest(value);
        let (l, r) = th.digest_leaf(path.as_bytes(), value_hash.as_bytes());
        current_hash = l;
        current_data = r;
        updates.push(vec![current_hash.as_bytes().to_vec(), current_data.to_vec()]);
    }

    for i in 0..proof.side_nodes.len() {
        let node = proof.side_nodes[i];

        if get_bits_at_from_msb(path.as_bytes(), proof.side_nodes.len() - 1 - i) == 1 {
            let (l, r) = th.digest_node(node.as_bytes(), current_hash.as_bytes());
            current_hash = l;
            current_data = r;
        } else {
            let (l, r) = th.digest_node(current_hash.as_bytes(), node.as_bytes());
            current_hash = l;
            current_data = r;
        }
        updates.push(vec![current_hash.as_bytes().to_vec(), current_data.to_vec()]);
    }


    return (current_hash.eq(&root), updates);
}

#[cfg(test)]
mod tests {
    use crypto::SHA256;
    use crate::proof::verify_proof;
    use crate::smt::SMT;

    #[test]
    fn test_proof_basic() {
        let mut smt = SMT::in_memory(None);

    }
}