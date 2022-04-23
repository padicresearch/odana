use crate::store::{Database, DatabaseBackend};
use crate::treehasher::TreeHasher;
use crate::utils::{count_common_prefix, get_bits_at_from_msb};
use anyhow::{bail, ensure, Result};
use primitive_types::H256;
use std::path::Path;
use crate::error::Error;
use crate::proof::Proof;

pub struct SparseMerkleTree {
    th: TreeHasher,
    db: Database,
    root: H256,
}

impl SparseMerkleTree {
    pub fn open<P: AsRef<Path>>(path: P, root: Option<H256>) -> Result<Self> {
        Ok(Self {
            th: TreeHasher::new(),
            db: Database::open(path)?,
            root: root.unwrap_or_default(),
        })
    }


    pub fn in_memory(root: Option<H256>) -> Self {
        Self {
            th: TreeHasher::new(),
            db: Database::in_memory(),
            root: root.unwrap_or_default(),
        }
    }

    pub fn set_root(&mut self, new_root: H256) {
        self.root = new_root
    }

    pub fn root(&self) -> H256 {
        self.root
    }
    pub fn get<K>(&self, key: K) -> Result<Vec<u8>> where K: AsRef<[u8]> {
        let root = self.root();
        if root.is_zero() {
            return Ok(Vec::new());
        }

        let path = self.th.path(key.as_ref());
        self.db.values.get_or_default(path.as_bytes(), Vec::new())
    }

    pub fn update<K, V>(&mut self, key: K, value: V) -> Result<H256> where K: AsRef<[u8]>, V: AsRef<[u8]> {
        let new_root = self.update_for_root(key.as_ref(), value.as_ref(), self.root())?;
        self.set_root(new_root);
        return Ok(new_root);
    }

    fn update_for_root(&self, key: &[u8], value: &[u8], root: H256) -> Result<H256> {
        let path = self.th.path(key);
        let (side_nodes, path_nodes, old_lead_data, _) =
            self.side_nodes_for_root(&path, &root, false)?;

        let mut new_root = H256::zero();
        if value.is_empty() {
            new_root =
                self.delete_with_sides_nodes(&path, &side_nodes, &path_nodes, &old_lead_data)?;
            self.db.values.delete(path.as_bytes())?;
        } else {
            new_root = self.update_with_sides_nodes(
                &path,
                &value,
                &side_nodes,
                &path_nodes,
                &old_lead_data,
            )?;
        }
        Ok(new_root)
    }

    fn depth(&self) -> usize {
        self.th.path_size() * 8
    }

    fn side_nodes_for_root(
        &self,
        path: &H256,
        root: &H256,
        get_sibling_data: bool,
    ) -> Result<(Vec<H256>, Vec<H256>, Vec<u8>, Option<Vec<u8>>)> {
        let mut side_nodes = Vec::with_capacity(self.depth());
        let mut path_nodes = Vec::with_capacity(self.depth() + 1);
        path_nodes.push(root.clone());

        if root.is_zero() {
            return Ok((side_nodes, path_nodes, Vec::new(), None));
        }

        let mut current_data = self.db.nodes.get(root.as_ref())?;
        if self.th.is_leaf(&current_data) {
            return Ok((side_nodes, path_nodes, current_data, None));
        }

        let mut node_hash = H256::zero();
        let mut side_node = Vec::new();
        let mut sibling_data = Vec::new();

        for i in 0..self.depth() {
            let (left_node, right_node) = self.th.parse_node(&current_data);
            if get_bits_at_from_msb(path.as_bytes(), i) == 1 {
                side_node = left_node.to_vec();
                node_hash = H256::from_slice(right_node);
            } else {
                side_node = right_node.to_vec();
                node_hash = H256::from_slice(left_node);
            }

            side_nodes.push(H256::from_slice(&side_node));
            path_nodes.push(node_hash);

            if node_hash.is_zero() {
                current_data = Vec::new();
                break
            }

            current_data = self.db.nodes.get(node_hash.as_bytes())?;
            if self.th.is_leaf(&current_data) {
                break;
            }
        }

        if get_sibling_data {
            sibling_data = self.db.nodes.get(&side_node)?;
        }

        side_nodes.reverse();
        path_nodes.reverse();
        return Ok((side_nodes, path_nodes, current_data, Some(sibling_data)));
    }

    fn delete_with_sides_nodes(
        &self,
        path: &H256,
        side_nodes: &Vec<H256>,
        path_nodes: &Vec<H256>,
        old_leaf_data: &Vec<u8>,
    ) -> Result<H256> {
        if path_nodes[0].is_zero() {
            bail!(Error::KeyAlreadyEmpty)
        }

        let (actual_path, _) = self.th.parse_leaf(old_leaf_data);
        if !actual_path.eq(path.as_bytes()) {
            bail!(Error::KeyAlreadyEmpty)
        }
        for node in path_nodes {
            self.db.nodes.delete(node.as_bytes())?;
        }

        let mut current_hash = H256::zero();
        let mut current_data = Vec::new();
        let mut non_placeholder_reached = false;

        for (i, side_node) in side_nodes.iter().enumerate() {
            if current_data.is_empty() {
                let side_node_value = self.db.nodes.get(side_node.as_bytes())?;
                if self.th.is_leaf(&side_node_value) {
                    current_hash = side_node.clone();
                    current_data = side_node.as_bytes().to_vec();
                    continue;
                } else {
                    current_data = self.th.placeholder().as_bytes().to_vec();
                    non_placeholder_reached = true;
                }
            }
            if !non_placeholder_reached && side_node.eq(&self.th.placeholder()) {
                continue;
            } else if !non_placeholder_reached {
                non_placeholder_reached = true
            }

            if get_bits_at_from_msb(path.as_bytes(), side_nodes.len() - 1 - i) == 1 {
                let (c, t) = self.th.digest_node(side_node.as_bytes(), &current_data);
                current_hash = c;
                current_data = t;
            } else {
                let (c, t) = self.th.digest_node(&current_data, side_node.as_bytes());
                current_hash = c;
                current_data = t;
            }
            self.db.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        }

        return Ok(current_hash);
    }

    fn update_with_sides_nodes(
        &self,
        path: &H256,
        value: &[u8],
        side_nodes: &Vec<H256>,
        path_nodes: &Vec<H256>,
        old_leaf_data: &Vec<u8>,
    ) -> Result<H256> {
        let value_hash = self.th.digest(value);
        let (mut current_hash, mut current_data) =
            self.th.digest_leaf(path.as_bytes(), value_hash.as_bytes());
        self.db.nodes.put(current_hash.as_bytes(), &current_data)?;
        current_data = current_hash.as_bytes().to_vec();

        let mut common_prefix_count = 0;
        let mut old_value_hash = None;

        if path_nodes[0].is_zero() {
            common_prefix_count = self.depth();
        } else {
            let mut actual_path = H256::zero();
            let (ap, op) = self.th.parse_leaf(old_leaf_data);
            actual_path = H256::from_slice(ap);
            old_value_hash = Some(H256::from_slice(op));
            common_prefix_count = count_common_prefix(path.as_bytes(), actual_path.as_bytes()) as usize;
        }

        if common_prefix_count != self.depth() {
            if get_bits_at_from_msb(path.as_bytes(), common_prefix_count) == 1 {
                (current_hash, current_data) = self
                    .th
                    .digest_node(path_nodes[0].as_bytes(), current_data.as_slice());
            } else {
                (current_hash, current_data) = self
                    .th
                    .digest_node(current_data.as_slice(), path_nodes[0].as_bytes());
            }
            self.db.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        } else if let Some(old_value_hash) = old_value_hash {
            if old_value_hash == value_hash {
                return Ok(self.root);
            }

            self.db.nodes.delete(path_nodes[0].as_bytes())?;
            self.db.values.delete(path.as_bytes())?;
        }

        for i in 1..path_nodes.len() {
            self.db.nodes.delete(path_nodes[i].as_bytes())?;
        }

        let offset_side_nodes = (self.depth() - side_nodes.len()) as i32;

        for i in 0..self.depth() {
            let mut side_node = H256::zero();
            if i as i32 - offset_side_nodes < 0 || side_nodes.get(i - offset_side_nodes as usize).is_none() {
                if common_prefix_count != self.depth() && common_prefix_count > self.depth() - 1 - i
                {
                    side_node = self.th.placeholder();
                } else {
                    continue;
                }
            } else {
                side_node = side_nodes[i - offset_side_nodes as usize];
            }

            if get_bits_at_from_msb(path.as_bytes(), self.depth() - 1 - i) == 1 {
                let (c, t) = self.th.digest_node(side_node.as_bytes(), &current_data);
                current_hash = c;
                current_data = t;
            } else {
                let (c, t) = self.th.digest_node(&current_data, side_node.as_bytes());
                current_hash = c;
                current_data = t;
            }

            self.db.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        }
        self.db.values.put(path.as_bytes(), value)?;
        Ok((current_hash))
    }

    pub fn proof(&self, key: &[u8]) -> Result<Proof> {
        return self.proof_for_root(key, &self.root)
    }

    pub fn proof_updatable(&self, key: &[u8]) -> Result<Proof> {
        return self.proof_updatable_for_root(key, &self.root)
    }

    pub fn proof_for_root(&self, key: &[u8], root: &H256) -> Result<Proof> {
        return self.do_proof_for_root(key, root, false)
    }

    pub fn proof_updatable_for_root(&self, key: &[u8], root: &H256) -> Result<Proof> {
        return self.do_proof_for_root(key, root, true)
    }

    fn do_proof_for_root(&self, key: &[u8], root: &H256, is_updatable: bool) -> Result<Proof> {
        let path = self.th.path(key);
        let (side_nodes, path_nodes, lead_data, sibling_data) =
            self.side_nodes_for_root(&path, &root, is_updatable)?;
        let mut non_empty_side_nodes = Vec::new();
        for v in side_nodes {
            non_empty_side_nodes.push(v)
        }

        let mut non_membership_leaf_data = None;
        if !path_nodes[0].is_zero() {
            let (actual_path, _) = self.th.parse_leaf(&lead_data);
            if !actual_path.eq(path.as_bytes()) {
                non_membership_leaf_data = Some(lead_data)
            }
        }

        Ok(Proof {
            side_nodes: non_empty_side_nodes,
            non_membership_leaf_data,
            sibling_data,
        })
    }
}


#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use crate::smt::SparseMerkleTree;

    #[test]
    fn basic_get_set_check_root_test() {
        let mut trie = SparseMerkleTree::in_memory(None);
        trie.update(b"kwame", b"AMA").unwrap();
        trie.update(b"kofi", b"AMA").unwrap();
        println!("{:?}", trie.root());
        println!("{:?}", trie.get(b"kofi").unwrap());
        println!("{:?}", trie.get(b"kwame").unwrap());
        trie.update(b"kofi", b"").unwrap();
        println!("{:?}", trie.root());

        println!("{:?}", trie.get(b"kofi").unwrap());
        println!("{:?}", trie.get(b"kwame").unwrap());
    }
}