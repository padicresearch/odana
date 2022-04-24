use std::fmt::{Debug};
use serde::{Serialize, Deserialize};
use primitive_types::H256;
use crate::treehasher::TreeHasher;
use crate::utils::get_bits_at_from_msb;
use hex::ToHex;
use crate::error::Error;
//use anyhow::Result;

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

pub fn verify_proof(proof: &Proof, root: H256, key: &[u8], value: &[u8]) -> bool {
    return verify_proof_with_updates(proof, root, key, value).is_ok();
}

struct Hasher;

impl TreeHasher for Hasher {}

pub(crate) fn verify_proof_with_updates(proof: &Proof, root: H256, key: &[u8], value: &[u8]) -> Result<Vec<Vec<Vec<u8>>>, Error> {
    let th = Hasher;
    let path = th.path(key);
    let mut updates = Vec::new();

    let mut current_hash = H256::zero();
    let mut current_data = Vec::new();

    if value.is_empty() {
        if proof.non_membership_leaf_data.is_none() {
            current_hash = th.placeholder()
        } else if let Some(non_membership_leaf_data) = &proof.non_membership_leaf_data {
            let (actual_path, value_hash) = th.parse_leaf(non_membership_leaf_data);
            if actual_path.eq(path.as_bytes()) {
                return Err(Error::NonMembershipPathError(actual_path.encode_hex::<String>(), path.as_bytes().encode_hex::<String>()))
            }
            let (l, r) = th.digest_leaf(actual_path, value_hash);
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

    if current_hash.ne(&root) {
        return Err(Error::BadProof(updates))
    }
    return Ok(updates);
}

#[cfg(test)]
mod tests {
    use primitive_types::H256;
    use crate::proof::verify_proof;
    use crate::smt::SparseMerkleTree;

    #[test]
    fn test_proof_basic() {
        let mut smt = SparseMerkleTree::default();

        // Generate and verify a proof on an empty key.
        let proof = smt.proof(b"testKey3").unwrap();
        let result = verify_proof(&proof, H256::zero(), b"testKey3", &Vec::new());
        assert!(result, "valid proof on empty key failed to verify");
        let result = verify_proof(&proof, H256::zero(), b"testKey3", b"badValue");
        assert!(!result, "invalid proof verification returned true");

        // Add a key, generate and verify a Merkle proof.
        let root = smt.update(b"testKey", b"testValue").unwrap();
        let proof = smt.proof(b"testKey").unwrap();
        let result = verify_proof(&proof, root, b"testKey", b"testValue");
        assert!(result, "valid proof failed to verify");
        let result = verify_proof(&proof, root, b"testKey", b"badValue");
        assert!(!result, "invalid proof verification returned true");

        //  Add a key, generate and verify both Merkle proofs.
        let root = smt.update(b"testKey2", b"testValue").unwrap();
        let proof = smt.proof(b"testKey").unwrap();
        let result = verify_proof(&proof, root, b"testKey", b"testValue");
        assert!(result, "valid proof failed to verify");
        let result = verify_proof(&proof, root, b"testKey", b"badValue");
        assert!(!result, "invalid proof verification returned true");
    }
}