use std::hash::{Hash, Hasher};

use bloomfilter::Bloom;

use crate::errors::*;
use crate::hasher::{HashFunction, Sha3Keccak256, HASH_LEN};

mod errors;
mod hasher;

const BITMAP_SIZE: usize = 32 * 1024 * 1024;

pub type MerkleRoot = [u8; HASH_LEN];

#[derive(Debug, Clone)]
pub struct Leave([u8; HASH_LEN]);

impl AsRef<[u8; HASH_LEN]> for Leave {
    fn as_ref(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

impl From<[u8; HASH_LEN]> for Leave {
    fn from(hash: [u8; 32]) -> Self {
        Leave(hash)
    }
}

impl Hash for Leave {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}

impl PartialEq for Leave {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other.as_ref())
    }
}

impl Eq for Leave {}

///
/// # Merkle Tree
///
#[derive(Debug)]
pub struct Merkle<H>
where
    H: HashFunction,
{
    root: Option<MerkleRoot>,
    pre_leaves_len: usize,
    leaves: Vec<Leave>,
    bloom_filter: Bloom<[u8]>,
    hasher: H,
}

impl Default for Merkle<Sha3Keccak256> {
    fn default() -> Self {
        Merkle {
            pre_leaves_len: 0,
            root: None,
            leaves: Vec::new(),
            bloom_filter: Bloom::new(BITMAP_SIZE, 1000),
            hasher: Sha3Keccak256,
        }
    }
}

impl<H: HashFunction> Merkle<H> {
    pub fn new(hasher: H, capacity: usize) -> Self {
        Merkle {
            pre_leaves_len: 0,
            root: None,
            leaves: Vec::with_capacity(capacity),
            bloom_filter: Bloom::new(BITMAP_SIZE, capacity),
            hasher,
        }
    }
}

impl<H: HashFunction> Merkle<H> {
    pub fn update(&mut self, item: &[u8]) -> Result<[u8; HASH_LEN], MerkleError> {
        if !self.bloom_filter.check(item) {
            let hash = self.hasher.digest(item);
            let leave = Leave(hash);
            self.leaves.push(leave);
            self.bloom_filter.set(item);
            return Ok(hash);
        }

        Err(MerkleError::MerkleTreeUpdateError)
    }

    pub fn finalize(&mut self) -> Option<&[u8; HASH_LEN]> {
        if self.pre_leaves_len < self.leaves.len() {
            self.calculate_root();
            self.pre_leaves_len = self.leaves.len();
        }

        match &self.root {
            None => None,
            Some(root) => Some(root),
        }
    }

    fn calculate_root(&mut self) {
        self.root = self._calculate_root(&self.leaves)
    }

    fn _calculate_root(&self, leaves: &[Leave]) -> Option<[u8; HASH_LEN]> {
        if leaves.is_empty() {
            return None;
        }
        let chucks = leaves.chunks(2);
        if chucks.len() == 1 {
            let c = chucks.into_iter().next().unwrap();
            let left = &c[0];
            let right = c.get(1).unwrap_or(left);
            let p = hash_pair(&self.hasher, (left.as_ref(), right.as_ref()));
            return Some(p);
        }
        let mut next = Vec::with_capacity(leaves.len() / 2);
        for c in chucks {
            let left = &c[0];
            let right = c.get(1).unwrap_or(left);
            let hash = hash_pair(&self.hasher, (left.as_ref(), right.as_ref()));
            next.push(Leave(hash))
        }

        self._calculate_root(&next)
    }

    pub fn proof(&self, item: &[u8]) -> Vec<(usize, (Leave, Leave))> {
        if !self.bloom_filter.check(item) {
            return vec![];
        }
        let hash = self.hasher.digest(item);
        let mut proofs = Vec::new();
        let valid_leave = Leave(hash);
        self._proof(valid_leave, &self.leaves, &mut proofs);
        proofs
    }

    fn _proof(&self, item: Leave, leaves: &[Leave], proof: &mut Vec<(usize, (Leave, Leave))>) {
        if leaves.is_empty() {
            return;
        }
        if leaves.len() == 1 {
            return;
        }

        let chucks = leaves.chunks(2);
        let mut next = Vec::new();
        let mut item = item;
        for c in chucks {
            let left = &c[0];
            let right = c.get(1).unwrap_or(left);
            let hash = hash_pair(&self.hasher, (left.as_ref(), right.as_ref()));
            if &item == left {
                proof.push((0, (left.clone(), right.clone())));
                item = Leave(hash);
            } else if &item == right {
                proof.push((1, (left.clone(), right.clone())));
                item = Leave(hash);
            }
            next.push(Leave(hash))
        }
        self._proof(item, &next, proof)
    }

    pub fn validate_proof(
        &self,
        item: &[u8],
        proof: &[(usize, (Leave, Leave))],
    ) -> Option<[u8; HASH_LEN]> {
        if !self.bloom_filter.check(item) {
            return None;
        }

        let valid_leaf = self.hasher.digest(item);

        let root = proof.iter().fold(valid_leaf, |root, (idx, pair)| {
            if *idx == 1 {
                hash_pair(&self.hasher, (pair.0.as_ref(), &root))
            } else {
                hash_pair(&self.hasher, (&root, pair.1.as_ref()))
            }
        });

        Some(root)
    }
}

pub fn hash_pair(h: &dyn HashFunction, pair: (&[u8; HASH_LEN], &[u8; HASH_LEN])) -> [u8; HASH_LEN] {
    let union_capacity = pair.0.len() + pair.1.len();
    let mut union = Vec::with_capacity(union_capacity);
    union.extend_from_slice(pair.0);
    union.extend_from_slice(pair.1);
    h.digest(union.as_slice())
}

#[cfg(test)]
mod tests {
    use crate::hasher::Sha3Keccak256;
    use crate::{hash_pair, HashFunction, Merkle};

    #[test]
    fn test_with_even_inputs() {
        let mut merkle = Merkle::default();
        merkle.update("hello".as_bytes());
        merkle.update("world".as_bytes());
        merkle.update("job".as_bytes());
        merkle.update("market".as_bytes());
        let root = merkle.finalize();

        let hasher = Sha3Keccak256;
        let h_a = hasher.digest("hello".as_bytes());
        let h_b = hasher.digest("world".as_bytes());
        let h_c = hasher.digest("job".as_bytes());
        let h_d = hasher.digest("market".as_bytes());

        let h_a_b = hash_pair(&hasher, (&h_a, &h_b));
        let h_c_d = hash_pair(&hasher, (&h_c, &h_d));
        let h_a_b_c_d = hash_pair(&hasher, (&h_a_b, &h_c_d));
        let merkle_root = root.unwrap();

        assert_eq!(*merkle_root, h_a_b_c_d);
        println!("{:?}", merkle_root);
        println!("{:?}", h_a_b_c_d);
    }

    #[test]
    fn test_proof() {
        let mut merkle = Merkle::default();
        merkle.update("hello".as_bytes());
        merkle.update("world".as_bytes());
        merkle.update("job".as_bytes());
        merkle.update("market".as_bytes());
        merkle.update("king".as_bytes());
        merkle.update("queen".as_bytes());
        merkle.update("baby".as_bytes());
        let root = merkle.finalize();
        let merkle_root = *root.unwrap();

        let proof = merkle.proof("baby".as_bytes());
        assert_eq!(
            merkle_root,
            merkle.validate_proof("baby".as_bytes(), &proof).unwrap()
        )
    }

    #[test]
    fn test_error_for_already_inserted_item() {
        let mut merkle = Merkle::default();
        merkle.update("hello".as_bytes());
        assert!(merkle.update("hello".as_bytes()).is_err());
    }

    #[test]
    fn test_root_odd_inputs() {
        let mut merkle = Merkle::default();
        merkle.update("hello".as_bytes());
        merkle.update("world".as_bytes());
        merkle.update("job".as_bytes());
        merkle.update("market".as_bytes());
        merkle.update("great".as_bytes());
        let root = merkle.finalize();

        let hasher = Sha3Keccak256;
        let h_a = hasher.digest("hello".as_bytes());
        let h_b = hasher.digest("world".as_bytes());
        let h_c = hasher.digest("job".as_bytes());
        let h_d = hasher.digest("market".as_bytes());
        let h_e = hasher.digest("great".as_bytes());
        let h_f = hasher.digest("great".as_bytes());

        let h_a_b = hash_pair(&hasher, (&h_a, &h_b));

        let h_c_d = hash_pair(&hasher, (&h_c, &h_d));

        let h_e_f = hash_pair(&hasher, (&h_e, &h_f));

        let h_g_h = hash_pair(&hasher, (&h_e, &h_f));

        let h_a_b_c_d = hash_pair(&hasher, (&h_a_b, &h_c_d));

        let h_e_f_g_h = hash_pair(&hasher, (&h_e_f, &h_g_h));

        let c_a_b_c_d_e_f_g_h = hash_pair(&hasher, (&h_a_b_c_d, &h_e_f_g_h));

        let merkle_root = root.unwrap();
        assert_eq!(merkle_root, &c_a_b_c_d_e_f_g_h);
    }
}
