use std::net::SocketAddr;
use serde::{Serialize, Deserialize};
use primitive_types::H256;
use crate::treehasher::TreeHasher;
use crate::utils::get_bits_at_from_msb;

#[derive(Serialize, Deserialize, Clone)]
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
        updates = Vec::with_capacity(2);
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
        updates = Vec::with_capacity(2);
        updates.push(vec![current_hash.as_bytes().to_vec(), current_data.to_vec()]);
    }


    return (current_hash.eq(&root), updates);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tempdir::TempDir;
    use primitive_types::H256;
    use crate::persistent::MemoryStore;
    use crate::proof::verify_proof;
    use crate::smt::SparseMerkleTree;
    use crate::store::Database;

    #[test]
    fn test_proof_basic() {
        let values = Arc::new(MemoryStore::new());
        let nodes = Arc::new(MemoryStore::new());
        let tmp_dir = TempDir::new("example").unwrap();
        let db = Database::test(nodes.clone(), values.clone());
        let mut smt = SparseMerkleTree::open(tmp_dir, None).unwrap();

        // Generate and verify a proof on an empty key.
        // let proof = smt.proof(b"testKey3").unwrap();
        // let result = verify_proof(proof.clone(),H256::zero(),b"testKey3", &Vec::new());
        // assert!(result, "valid proof on empty key failed to verify");
        // let result = verify_proof(proof,H256::zero(),b"testKey3", b"badValue");
        // assert!(!result, "invalid proof verification returned true");

        // Add a key, generate and verify a Merkle proof.
        let root = smt.update(b"testKey", b"testValue").unwrap();
        // let proof = smt.proof(b"testKey").unwrap();
        // let result = verify_proof(proof.clone(),root,b"testKey", b"testValue");
        // assert!(result,"valid proof failed to verify");
        // let result = verify_proof(proof,root,b"testKey", b"badValue");
        // assert!(!result, "invalid proof verification returned true");

        //  Add a key, generate and verify both Merkle proofs.
        let root = smt.update(b"testKey2", b"testValue").unwrap();
        //println!("{:#?}", values);
        //println!("{:#?}", nodes);
        let proof = smt.proof(b"testKey").unwrap();
        let result = verify_proof(proof.clone(), root, b"testKey", b"testValue");
        assert!(result, "valid proof failed to verify");
        let result = verify_proof(proof, root, b"testKey", b"badValue");
        assert!(!result, "invalid proof verification returned true");
    }
}