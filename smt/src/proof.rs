use alloc::vec;
use alloc::vec::Vec;

use bincode::{Decode, Encode};
use codec::{Decodable, Encodable};
use primitive_types::H256;

use crate::error::Error;
use crate::treehasher::TreeHasher;
use crate::utils::get_bits_at_from_msb;

#[derive(Clone, Debug, Encode, Decode)]
pub struct Proof {
    pub side_nodes: Vec<H256>,
    pub non_membership_leaf_data: Option<Vec<u8>>,
    pub sibling_data: Option<Vec<u8>>,
}

impl Encodable for Proof {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        bincode::encode_to_vec(self, codec::config()).map_err(|e| e.into())
    }
}

impl Decodable for Proof {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        bincode::decode_from_slice(buf, codec::config())
            .map(|(output, _)| output)
            .map_err(|e| e.into())
    }
}

pub struct CompatProof {
    pub side_nodes: Vec<H256>,
    pub non_membership_leaf_data: Vec<u8>,
    pub bitmask: Vec<u8>,
    pub num_side_nodes: u32,
    pub sibling_data: Option<Vec<u8>>,
}

pub fn verify_proof<T: TreeHasher>(
    hasher: &T,
    proof: &Proof,
    root: H256,
    key: &[u8],
    value: &[u8],
) -> bool {
    verify_proof_with_updates(hasher, proof, root, key, value).is_ok()
}

pub fn verify_proof_with_updates<T: TreeHasher>(
    hasher: &T,
    proof: &Proof,
    root: H256,
    key: &[u8],
    value: &[u8],
) -> Result<Vec<Vec<Vec<u8>>>, Error> {
    let th = hasher;
    let path = th.path(key);
    let mut updates = Vec::new();

    let mut current_hash = H256::zero();

    if value.is_empty() {
        if proof.non_membership_leaf_data.is_none() {
            current_hash = th.placeholder()
        } else if let Some(non_membership_leaf_data) = &proof.non_membership_leaf_data {
            let (actual_path, value_hash) = th.parse_leaf(non_membership_leaf_data);
            if actual_path.eq(path.as_bytes()) {
                return Err(Error::NonMembershipPathError(
                    actual_path.to_vec(),
                    path.as_bytes().to_vec(),
                ));
            }
            let (l, current_data) = th.digest_leaf(actual_path, value_hash);
            current_hash = l;
            updates.push(vec![
                current_hash.as_bytes().to_vec(),
                current_data.to_vec(),
            ]);
        }
    } else {
        let value_hash = th.digest(value);
        let (l, current_data) = th.digest_leaf(path.as_bytes(), value_hash.as_bytes());
        current_hash = l;
        updates.push(vec![
            current_hash.as_bytes().to_vec(),
            current_data.to_vec(),
        ]);
    }

    for i in 0..proof.side_nodes.len() {
        let node = proof.side_nodes[i];
        let current_data =
            if get_bits_at_from_msb(path.as_bytes(), proof.side_nodes.len() - 1 - i) == 1 {
                let (l, r) = th.digest_node(node.as_bytes(), current_hash.as_bytes());
                current_hash = l;
                r
            } else {
                let (l, r) = th.digest_node(current_hash.as_bytes(), node.as_bytes());
                current_hash = l;
                r
            };
        updates.push(vec![current_hash.as_bytes().to_vec(), current_data]);
    }

    if current_hash.ne(&root) {
        return Err(Error::BadProof(updates));
    }
    Ok(updates)
}

#[cfg(test)]
mod tests {
    use crate::proof::verify_proof;
    use crate::smt::SparseMerkleTree;
    use crate::{DefaultTreeHasher, StorageBackend, StorageBackendSnapshot};
    use alloc::vec::Vec;
    use anyhow::{bail, Result};
    use dashmap::DashMap;
    use primitive_types::H256;

    #[test]
    fn test_proof_basic() {
        let mut smt = SparseMerkleTree::new();

        let hasher = smt.hasher.clone();

        // Generate and verify a proof on an empty key.
        let proof = smt.proof(b"testKey3").unwrap();
        let result = verify_proof(&hasher, &proof, H256::zero(), b"testKey3", &Vec::new());
        assert!(result, "valid proof on empty key failed to verify");
        let result = verify_proof(&hasher, &proof, H256::zero(), b"testKey3", b"badValue");
        assert!(!result, "invalid proof verification returned true");

        // Add a key, generate and verify a Merkle proof.
        let root = smt.update(b"testKey", b"testValue").unwrap();
        let proof = smt.proof(b"testKey").unwrap();
        let result = verify_proof(&hasher, &proof, root, b"testKey", b"testValue");
        assert!(result, "valid proof failed to verify");
        let result = verify_proof(&hasher, &proof, root, b"testKey", b"badValue");
        assert!(!result, "invalid proof verification returned true");

        //  Add a key, generate and verify both Merkle proofs.
        let root = smt.update(b"testKey2", b"testValue").unwrap();
        let proof = smt.proof(b"testKey").unwrap();
        let result = verify_proof(&hasher, &proof, root, b"testKey", b"testValue");
        assert!(result, "valid proof failed to verify");
        let result = verify_proof(&hasher, &proof, root, b"testKey", b"badValue");
        assert!(!result, "invalid proof verification returned true");
    }
}
