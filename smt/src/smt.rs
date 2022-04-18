use crate::store::{Database, DatabaseBackend, StorageError};
use crate::treehasher::TreeHasher;
use crate::utils::{count_common_prefix, get_bits_at_from_msb};
use anyhow::{bail, ensure, Result};
use primitive_types::H256;
use std::path::Path;

pub struct Trie {
    th: TreeHasher,
    db: Database,
    root: H256,
}

impl Trie {
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
    pub fn get(&self, key: &Vec<u8>) -> Result<Vec<u8>> {
        let root = self.root();
        if root.is_zero() {
            return Ok(Vec::new());
        }

        let path = self.th.path(&key);
        self.db.value.get_or_default(path.as_bytes(), Vec::new())
    }

    pub fn update(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<H256> {
        let new_root = self.update_for_root(key, value, self.root())?;
        self.set_root(new_root);
        return Ok(new_root);
    }

    fn update_for_root(&mut self, key: Vec<u8>, value: Vec<u8>, root: H256) -> Result<H256> {
        let path = self.th.path(&key);
        let (side_nodes, path_nodes, old_lead_data, _) =
            self.side_nodes_for_root(&path, &root, false)?;

        let mut new_root = H256::zero();
        if value.is_empty() {
            new_root =
                self.delete_with_sides_nodes(&path, &side_nodes, &path_nodes, &old_lead_data)?;
            self.db.nodes.delete(path.as_bytes())?;
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
        self.th.path_size()
    }

    fn side_nodes_for_root(
        &mut self,
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

        let mut node_hash = Vec::new();
        let mut side_node = Vec::new();
        let mut sibling_data = Vec::new();

        for i in 0..self.depth() {
            let (left_node, right_node) = self.th.parse_node(&current_data);
            if get_bits_at_from_msb(path.as_bytes(), i) == 1 {
                side_node = left_node.to_vec();
                node_hash = right_node.to_vec();
            } else {
                side_node = right_node.to_vec();
                node_hash = left_node.to_vec();
            }

            side_nodes.push(H256::from_slice(&side_node));
            path_nodes.push(H256::from_slice(&node_hash));

            current_data = self.db.nodes.get(&node_hash)?;
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
        &mut self,
        path: &H256,
        side_nodes: &Vec<H256>,
        path_nodes: &Vec<H256>,
        old_leaf_data: &Vec<u8>,
    ) -> Result<H256> {
        if path_nodes[0].is_zero() {
            bail!("errKeyAlreadyEmpty")
        }

        let (actual_path, _) = self.th.parse_leaf(old_leaf_data);
        ensure!(path.as_bytes() != actual_path, "errKeyAlreadyEmpty");
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
        &mut self,
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
        let mut old_value_hash = H256::zero();

        if path_nodes[0].is_zero() {
            common_prefix_count = self.depth();
        } else {
            let mut actual_path = H256::zero();
            let (ap, op) = self.th.parse_leaf(old_leaf_data);
            actual_path = H256::from_slice(ap);
            old_value_hash = H256::from_slice(op);
            common_prefix_count = count_common_prefix(ap, op) as usize;
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
        } else if !old_value_hash.is_zero() {
            if old_value_hash == value_hash {
                return Ok(self.root);
            }

            self.db.nodes.delete(path_nodes[0].as_bytes())?;
            self.db.value.delete(path.as_bytes())?;
        }

        for i in 0..path_nodes.len() {
            self.db.nodes.delete(path_nodes[i].as_bytes())?;
        }

        let offset_side_nodes = (self.depth() - side_nodes.len()) as i32;

        for i in 0..self.depth() {
            let mut side_node = H256::zero();
            if i as i32 - offset_side_nodes < 0 || side_nodes.get(i - offset_side_nodes as usize).is_some() {
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
        self.db.value.put(path.as_bytes(), value)?;
        Ok((current_hash))
    }
}


#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use crate::smt::Trie;

    #[test]
    fn basic_get_set_check_root_test() {
        let mut trie = Trie::in_memory(None);
        println!("{}", trie.root());
        trie.update(vec![1, 2, 3], vec![1, 2, 3]).unwrap();
        println!("{}", trie.root());
        println!("{:?}", trie.get(&vec![1, 2, 3]).unwrap())
    }
}