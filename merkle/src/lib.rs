mod errors;

use linked_hash_set::LinkedHashSet;
use sha2::{Sha256, Digest};
use std::collections::HashSet;
use crate::errors::*;
use std::ops::Index;
use serde::{Serialize, Deserialize};
use hex;

pub trait HashFunction {
    fn digest(&self, input: &[u8]) -> String;
}

///
/// # Merkle Tree
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Merkle<H> where H: HashFunction {
    root: Option<String>,
    pre_leaves_len: usize,
    leaves: LinkedHashSet<String>,
    hasher: H,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SHA256Hasher {}

impl HashFunction for SHA256Hasher {
    fn digest(&self, input: &[u8]) -> String {
        let out = Sha256::digest(input.as_ref());
        hex::encode(out)
    }
}


impl Default for Merkle<SHA256Hasher> {
    fn default() -> Self {
        let hasher = SHA256Hasher {};
        Merkle {
            pre_leaves_len: 0,
            root: None,
            leaves: LinkedHashSet::new(),
            hasher,
        }
    }
}

impl<H: HashFunction> Merkle<H> {
    pub fn new(hasher: H) -> Self {
        Merkle {
            pre_leaves_len: 0,
            root: None,
            leaves: LinkedHashSet::new(),
            hasher,
        }
    }
}

impl<H: HashFunction> Merkle<H> {
    pub fn update(&mut self, item: &[u8]) -> Result<String, MerkleTreeUpdateError> {
        let hash = self.hasher.digest(item);
        match self.leaves.insert_if_absent(hash.clone()) {
            true => {
                Ok(hash)
            }
            false => {
                Err(MerkleTreeUpdateError)
            }
        }
    }

    pub fn finalize(&mut self) -> Option<String> {
        if self.pre_leaves_len < self.leaves.len() {
            self.calculate_root();
            self.pre_leaves_len = self.leaves.len();
        }

        match &self.root {
            None => {
                None
            }
            Some(root) => {
                Some(root.clone())
            }
        }
    }

    fn calculate_root(&mut self) {
        let mut items = vec![];
        items.extend(self.leaves.iter().map(|s| s.to_owned()));
        self.root = self._calculate_root(items)
    }

    fn _calculate_root(&self, items: Vec<String>) -> Option<String> {
        if items.is_empty() {
            return None;
        }
        let mut leaves = items;
        if leaves.len() % 2 != 0 {
            leaves.push(leaves.last().unwrap().to_owned())
        }
        let chucks = leaves.chunks_exact(2);
        //let d = chucks.nth(0);
        if chucks.len() == 1 {
            let c = chucks.into_iter().next().unwrap();
            let p = hash_pair(&self.hasher, (&c[0], &c[1]));
            return Some(p);
        }
        let mut leaves = Vec::new();
        for c in chucks {
            let hash = hash_pair(&self.hasher, (&c[0], &c[1]));
            leaves.push(hash)
        }

        self._calculate_root(leaves)
    }

    pub fn proof(&self, item: String, out: &mut Vec<(usize, (String, String))>) {
        let mut items = vec![];
        items.extend(self.leaves.iter().map(|s| s.to_owned()));
        self._proof(item, items, out);
    }

    fn _proof(&self, item: String, items: Vec<String>, proof: &mut Vec<(usize, (String, String))>) {
        if items.is_empty() {
            return;
        }
        if items.len() == 1 {
            return;
        }
        let mut item = item;
        let mut leaves = items;
        if leaves.len() % 2 != 0 {
            leaves.push(leaves.last().unwrap().to_owned())
        }
        let chucks = leaves.chunks_exact(2);
        let mut leaves = Vec::new();
        for c in chucks {
            let hash = hash_pair(&self.hasher, (&c[0], &c[1]));
            if c.contains(&item) {
                let idx = c.iter().position(|i| item.eq(i)).unwrap();
                proof.push((idx, (c[0].to_owned(), c[1].to_owned())));
                item = hash.clone();
            }
            leaves.push(hash)
        }
        self._proof(item, leaves, proof)
    }

    pub fn validate_proof(&self, item: String, proof: &Vec<(usize, (String, String))>) -> String {
        let root = proof.iter().fold(item, |root, (idx, pair)| {
            if *idx == 1 {
                hash_pair(&self.hasher, (&pair.0, &root))
            } else {
                hash_pair(&self.hasher, (&root, &pair.1))
            }
        });

        root.to_owned()
    }
}

pub fn hash_pair(h: &dyn HashFunction, pair: (&str, &str)) -> String {
    let p: String = format!("{}{}", pair.0, pair.1);
    h.digest(p.as_bytes())
}


#[cfg(test)]
mod tests {
    use crate::{Merkle, HashFunction, hash_pair, SHA256Hasher};

    #[test]
    fn test_with_even_inputs() {
        let mut merkle = Merkle::default();
        merkle.update("hello".as_bytes());
        merkle.update("world".as_bytes());
        merkle.update("job".as_bytes());
        merkle.update("market".as_bytes());
        let root = merkle.finalize();

        let hasher = SHA256Hasher {};
        let h_a = hasher.digest("hello".as_bytes());
        let h_b = hasher.digest("world".as_bytes());
        let h_c = hasher.digest("job".as_bytes());
        let h_d = hasher.digest("market".as_bytes());

        let c_a_b = format!("{}{}", h_a, h_b);
        let h_a_b = hasher.digest(c_a_b.as_bytes());

        let c_c_d = format!("{}{}", h_c, h_d);
        let h_c_d = hasher.digest(c_c_d.as_bytes());

        let c_a_b_c_d = format!("{}{}", h_a_b, h_c_d);
        let h_a_b_c_d = hasher.digest(c_a_b_c_d.as_bytes());

        let merkle_root = root.unwrap();

        assert_eq!(merkle_root, h_a_b_c_d);
        println!("{:#?}", merkle_root);
        println!("{:#?}", h_a_b_c_d);
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
        let merkle_root = root.unwrap();


        let hasher = SHA256Hasher {};
        let item = hasher.digest("baby".as_bytes());
        let mut proof = Vec::new();
        merkle.proof(item.clone(), &mut proof);
        println!("Merkel Root: {:#?}", merkle_root);
        assert_eq!(merkle_root, merkle.validate_proof(item, &proof))
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

        let hasher = SHA256Hasher {};
        let h_a = hasher.digest("hello".as_bytes());
        let h_b = hasher.digest("world".as_bytes());
        let h_c = hasher.digest("job".as_bytes());
        let h_d = hasher.digest("market".as_bytes());
        let h_e = hasher.digest("great".as_bytes());
        let h_f = hasher.digest("great".as_bytes());

        let c_a_b = format!("{}{}", h_a, h_b);
        let h_a_b = hasher.digest(c_a_b.as_bytes());

        let c_c_d = format!("{}{}", h_c, h_d);
        let h_c_d = hasher.digest(c_c_d.as_bytes());

        let c_e_f = format!("{}{}", h_e, h_f);
        let h_e_f = hasher.digest(c_e_f.as_bytes());

        let c_g_h = format!("{}{}", h_e, h_f);
        let h_g_h = hasher.digest(c_g_h.as_bytes());


        let c_a_b_c_d = format!("{}{}", h_a_b, h_c_d);
        let h_a_b_c_d = hasher.digest(c_a_b_c_d.as_bytes());

        let c_e_f_g_h = format!("{}{}", h_e_f, h_g_h);
        let h_e_f_g_h = hasher.digest(c_e_f_g_h.as_bytes());

        let c_a_b_c_d_e_f_g_h = format!("{}{}", h_a_b_c_d, h_e_f_g_h);
        let c_a_b_c_d_e_f_g_h = hasher.digest(c_a_b_c_d_e_f_g_h.as_bytes());

        let merkle_root = root.unwrap();
        assert_eq!(merkle_root, c_a_b_c_d_e_f_g_h);
        println!("{:#?}", merkle_root);
        println!("{:#?}", c_a_b_c_d_e_f_g_h);
    }
}
