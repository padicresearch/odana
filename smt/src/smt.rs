use crate::Result;
use alloc::format;
use alloc::vec::Vec;
use codec::{Decodable, Encodable};
use primitive_types::H256;

use crate::error::Error;
use crate::proof::{verify_proof_with_updates, Proof};
use crate::treehasher::TreeHasher;
use crate::utils::{count_common_prefix, get_bits_at_from_msb};
use crate::{
    CopyStrategy, DefaultTreeHasher, MemoryStorage, StorageBackend, StorageBackendSnapshot,
};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

pub(crate) struct SideNodesForRootResult(Vec<H256>, Vec<H256>, Vec<u8>, Option<Vec<u8>>);

#[derive(Clone, Debug)]
pub struct SparseMerkleTree<Storage = MemoryStorage, Hasher = DefaultTreeHasher> {
    pub(crate) nodes: Storage,
    pub(crate) values: Storage,
    pub(crate) hasher: Hasher,
    pub(crate) root: H256,
    pub(crate) parent: H256,
}

impl<Storage: StorageBackend, Hasher: TreeHasher> Encode for SparseMerkleTree<Storage, Hasher> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> core::result::Result<(), EncodeError> {
        Encode::encode(
            &self
                .nodes
                .snapshot()
                .map_err(|e| EncodeError::OtherString(format!("{}", e)))?,
            encoder,
        )?;
        Encode::encode(
            &self
                .values
                .snapshot()
                .map_err(|e| EncodeError::OtherString(format!("{}", e)))?,
            encoder,
        )?;
        Encode::encode(&self.hasher, encoder)?;
        Encode::encode(&self.root, encoder)?;
        Encode::encode(&self.parent, encoder)?;
        Ok(())
    }
}

impl<Storage: StorageBackend, Hasher: TreeHasher> Decode for SparseMerkleTree<Storage, Hasher> {
    fn decode<D: Decoder>(decoder: &mut D) -> core::result::Result<Self, DecodeError> {
        let nodes_snapshot: StorageBackendSnapshot = bincode::Decode::decode(decoder)
            .map_err(|e| DecodeError::OtherString(format!("{}", e)))?;
        let values_snapshot: StorageBackendSnapshot = bincode::Decode::decode(decoder)
            .map_err(|e| DecodeError::OtherString(format!("{}", e)))?;
        let hasher: Hasher = Decode::decode(decoder)?;
        let root: H256 = bincode::Decode::decode(decoder)?;
        let parent: H256 = bincode::Decode::decode(decoder)?;
        let nodes = Storage::from_snapshot(nodes_snapshot)
            .map_err(|e| DecodeError::OtherString(format!("{}", e)))?;
        let values = Storage::from_snapshot(values_snapshot)
            .map_err(|e| DecodeError::OtherString(format!("{}", e)))?;

        Ok(Self {
            hasher,
            nodes,
            values,
            root,
            parent,
        })
    }
}

impl<Storage: StorageBackend, Hasher: TreeHasher> Encodable for SparseMerkleTree<Storage, Hasher> {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        bincode::encode_to_vec(self, bincode::config::standard()).map_err(|e| e.into())
    }
}

impl<Storage: StorageBackend, Hasher: TreeHasher> Decodable for SparseMerkleTree<Storage, Hasher> {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        bincode::decode_from_slice(buf, bincode::config::standard())
            .map(|(t, _)| t)
            .map_err(|e| e.into())
    }
}

impl SparseMerkleTree<MemoryStorage, DefaultTreeHasher> {
    pub fn new() -> Self {
        Self {
            nodes: MemoryStorage::new(),
            values: MemoryStorage::new(),
            hasher: DefaultTreeHasher,
            root: Default::default(),
            parent: Default::default(),
        }
    }
}

impl Default for SparseMerkleTree<MemoryStorage, DefaultTreeHasher> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Storage: StorageBackend, Hasher: TreeHasher> SparseMerkleTree<Storage, Hasher> {
    pub fn new_with_hasher(hasher: Hasher, nodes: Storage, values: Storage) -> Self {
        Self {
            nodes,
            values,
            hasher,
            root: Default::default(),
            parent: Default::default(),
        }
    }

    pub fn set_root(&mut self, new_root: H256) {
        self.root = new_root
    }

    pub fn subtree(&self, strategy: CopyStrategy, import_keys: Vec<Vec<u8>>) -> Result<Self> {
        let mut subtree =
            SparseMerkleTree::new_with_hasher(self.hasher.clone(), Storage::new(), Storage::new());
        subtree.parent = self.root;
        subtree.root = self.root;

        match strategy {
            CopyStrategy::Partial => {
                subtree.nodes = self.nodes.clone();
                for key in import_keys {
                    let (value, proof) = self.get_with_proof_updatable(&key)?;
                    subtree.add_branch(&proof, self.root(), &key, &value)?;
                }
            }
            CopyStrategy::Full => {
                subtree = self.clone();
            }
            CopyStrategy::None => {}
        }

        Ok(subtree)
    }

    pub fn add_branch(
        &mut self,
        proof: &Proof,
        root: H256,
        key: &[u8],
        value: &[u8],
    ) -> Result<()> {
        let updates = verify_proof_with_updates(&self.hasher, proof, root, key, value)?;
        if !value.is_empty() {
            self.values.put(self.hasher.path(key).as_bytes(), value)?;
        }

        for update in updates {
            self.nodes.put(&update[0], &update[1])?;
        }

        if let Some(sibling_data) = &proof.sibling_data {
            if !proof.side_nodes.is_empty() && !proof.side_nodes.is_empty() {
                self.nodes
                    .put(proof.side_nodes[0].as_bytes(), sibling_data)?;
            }
        }
        Ok(())
    }

    fn update_for_root(&mut self, key: &[u8], value: &[u8], root: H256) -> Result<H256> {
        let path = self.hasher.path(key);
        let SideNodesForRootResult(side_nodes, path_nodes, old_lead_data, _) =
            self.side_nodes_for_root(&path, &root, false)?;

        if value.is_empty() {
            self.values.delete(path.as_bytes())?;
            self.delete_with_sides_nodes(&path, &side_nodes, &path_nodes, &old_lead_data)
        } else {
            self.update_with_sides_nodes(&path, value, &side_nodes, &path_nodes, &old_lead_data)
        }
    }

    fn depth(&self) -> usize {
        self.hasher.path_size() * 8
    }

    fn side_nodes_for_root(
        &self,
        path: &H256,
        root: &H256,
        get_sibling_data: bool,
    ) -> Result<SideNodesForRootResult> {
        let mut side_nodes = Vec::with_capacity(self.depth());
        let mut path_nodes = Vec::with_capacity(self.depth() + 1);
        path_nodes.push(*root);

        if root.is_zero() {
            return Ok(SideNodesForRootResult(
                side_nodes,
                path_nodes,
                Vec::new(),
                None,
            ));
        }

        let mut current_data = self.nodes.get(root.as_ref())?;
        if self.hasher.is_leaf(&current_data) {
            return Ok(SideNodesForRootResult(
                side_nodes,
                path_nodes,
                current_data,
                None,
            ));
        }

        let mut side_node = Vec::new();
        let mut sibling_data = Vec::new();

        for i in 0..self.depth() {
            let (left_node, right_node) = self.hasher.parse_node(&current_data);
            let node_hash = if get_bits_at_from_msb(path.as_bytes(), i) == 1 {
                side_node = left_node.to_vec();
                H256::from_slice(right_node)
            } else {
                side_node = right_node.to_vec();
                H256::from_slice(left_node)
            };

            side_nodes.push(H256::from_slice(&side_node));
            path_nodes.push(node_hash);

            if node_hash.is_zero() {
                current_data = Vec::new();
                break;
            }

            current_data = self.nodes.get(node_hash.as_bytes())?;
            if self.hasher.is_leaf(&current_data) {
                break;
            }
        }

        if get_sibling_data {
            sibling_data = self.nodes.get(&side_node)?;
        }

        side_nodes.reverse();
        path_nodes.reverse();
        Ok(SideNodesForRootResult(
            side_nodes,
            path_nodes,
            current_data,
            Some(sibling_data),
        ))
    }

    fn delete_with_sides_nodes(
        &mut self,
        path: &H256,
        side_nodes: &Vec<H256>,
        path_nodes: &Vec<H256>,
        old_leaf_data: &[u8],
    ) -> Result<H256> {
        if path_nodes[0].is_zero() {
            return Err(Error::KeyAlreadyEmpty);
        }

        let (actual_path, _) = self.hasher.parse_leaf(old_leaf_data);
        if !actual_path.eq(path.as_bytes()) {
            return Err(Error::KeyAlreadyEmpty);
        }
        for node in path_nodes {
            self.nodes.delete(node.as_bytes())?;
        }

        let mut current_hash = H256::zero();
        let mut current_data = Vec::new();
        let mut non_placeholder_reached = false;

        for (i, side_node) in side_nodes.iter().enumerate() {
            if current_data.is_empty() {
                let side_node_value = self.nodes.get(side_node.as_bytes())?;
                if self.hasher.is_leaf(&side_node_value) {
                    current_hash = *side_node;
                    current_data = side_node.as_bytes().to_vec();
                    continue;
                } else {
                    current_data = self.hasher.placeholder().as_bytes().to_vec();
                    non_placeholder_reached = true;
                }
            }
            if !non_placeholder_reached && side_node.eq(&self.hasher.placeholder()) {
                continue;
            } else if !non_placeholder_reached {
                non_placeholder_reached = true
            }

            if get_bits_at_from_msb(path.as_bytes(), side_nodes.len() - 1 - i) == 1 {
                let (c, t) = self.hasher.digest_node(side_node.as_bytes(), &current_data);
                current_hash = c;
                current_data = t;
            } else {
                let (c, t) = self.hasher.digest_node(&current_data, side_node.as_bytes());
                current_hash = c;
                current_data = t;
            }
            self.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        }

        Ok(current_hash)
    }

    fn update_with_sides_nodes(
        &mut self,
        path: &H256,
        value: &[u8],
        side_nodes: &[H256],
        path_nodes: &[H256],
        old_leaf_data: &[u8],
    ) -> Result<H256> {
        let value_hash = self.hasher.digest(value);
        let (mut current_hash, mut current_data) = self
            .hasher
            .digest_leaf(path.as_bytes(), value_hash.as_bytes());
        self.nodes.put(current_hash.as_bytes(), &current_data)?;
        current_data = current_hash.as_bytes().to_vec();

        let mut old_value_hash = None;

        let common_prefix_count = if path_nodes[0].is_zero() {
            self.depth()
        } else {
            let (ap, op) = self.hasher.parse_leaf(old_leaf_data);
            let actual_path = H256::from_slice(ap);
            old_value_hash = Some(H256::from_slice(op));
            count_common_prefix(path.as_bytes(), actual_path.as_bytes()) as usize
        };

        if common_prefix_count != self.depth() {
            if get_bits_at_from_msb(path.as_bytes(), common_prefix_count) == 1 {
                (current_hash, current_data) = self
                    .hasher
                    .digest_node(path_nodes[0].as_bytes(), current_data.as_slice());
            } else {
                (current_hash, current_data) = self
                    .hasher
                    .digest_node(current_data.as_slice(), path_nodes[0].as_bytes());
            }
            self.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        } else if let Some(old_value_hash) = old_value_hash {
            if old_value_hash == value_hash {
                return Ok(self.root);
            }

            self.nodes.delete(path_nodes[0].as_bytes())?;
            //Error not handled because function fails when SMT is a Subtrees with no values copied
            let _ = self.values.delete(path.as_bytes());
        }

        for node in path_nodes.iter().skip(1) {
            self.nodes.delete(node.as_bytes())?;
        }

        let offset_side_nodes = (self.depth() - side_nodes.len()) as i32;

        for i in 0..self.depth() {
            let side_node = if i as i32 - offset_side_nodes < 0
                || side_nodes.get(i - offset_side_nodes as usize).is_none()
            {
                if common_prefix_count != self.depth() && common_prefix_count > self.depth() - 1 - i
                {
                    self.hasher.placeholder()
                } else {
                    continue;
                }
            } else {
                side_nodes[i - offset_side_nodes as usize]
            };

            if get_bits_at_from_msb(path.as_bytes(), self.depth() - 1 - i) == 1 {
                let (c, t) = self.hasher.digest_node(side_node.as_bytes(), &current_data);
                current_hash = c;
                current_data = t;
            } else {
                let (c, t) = self.hasher.digest_node(&current_data, side_node.as_bytes());
                current_hash = c;
                current_data = t;
            }

            self.nodes.put(current_hash.as_bytes(), &current_data)?;
            current_data = current_hash.as_bytes().to_vec();
        }
        self.values.put(path.as_bytes(), value)?;
        Ok(current_hash)
    }

    pub fn proof(&self, key: &[u8]) -> Result<Proof> {
        self.proof_for_root(key, &self.root)
    }

    pub fn proof_updatable(&self, key: &[u8]) -> Result<Proof> {
        self.proof_updatable_for_root(key, &self.root)
    }

    pub fn proof_for_root(&self, key: &[u8], root: &H256) -> Result<Proof> {
        self.do_proof_for_root(key, root, false)
    }

    pub fn proof_updatable_for_root(&self, key: &[u8], root: &H256) -> Result<Proof> {
        self.do_proof_for_root(key, root, true)
    }

    pub fn get_with_proof_for_root(&self, key: &[u8], root: &H256) -> Result<(Vec<u8>, Proof)> {
        let value = self.get(key)?;
        let proof = self.do_proof_for_root(key, root, false)?;
        Ok((value, proof))
    }

    pub fn get_with_proof_updatable_for_root(
        &self,
        key: &[u8],
        root: &H256,
    ) -> Result<(Vec<u8>, Proof)> {
        let value = self.get(key)?;
        let proof = self.do_proof_for_root(key, root, true)?;
        Ok((value, proof))
    }

    fn do_proof_for_root(&self, key: &[u8], root: &H256, is_updatable: bool) -> Result<Proof> {
        let path = self.hasher.path(key);
        let SideNodesForRootResult(side_nodes, path_nodes, lead_data, sibling_data) =
            self.side_nodes_for_root(&path, root, is_updatable)?;
        let mut non_empty_side_nodes = Vec::new();
        for v in side_nodes {
            non_empty_side_nodes.push(v)
        }

        let mut non_membership_leaf_data = None;
        if !path_nodes[0].is_zero() {
            let (actual_path, _) = self.hasher.parse_leaf(&lead_data);
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

    pub fn get<K>(&self, key: K) -> Result<Vec<u8>>
    where
        K: AsRef<[u8]>,
    {
        let root = self.root();
        if root.is_zero() {
            return Ok(Vec::new());
        }

        let path = self.hasher.path(key.as_ref());
        self.values.get_or_default(path.as_bytes(), Vec::new())
    }

    pub fn get_with_proof<K>(&self, key: K) -> Result<(Vec<u8>, Proof)>
    where
        K: AsRef<[u8]>,
    {
        return self.get_with_proof_for_root(key.as_ref(), &self.root());
    }

    pub fn get_with_proof_updatable<K>(&self, key: K) -> Result<(Vec<u8>, Proof)>
    where
        K: AsRef<[u8]>,
    {
        return self.get_with_proof_updatable_for_root(key.as_ref(), &self.root());
    }

    pub fn update<K, V>(&mut self, key: K, value: V) -> Result<H256>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let new_root = self.update_for_root(key.as_ref(), value.as_ref(), self.root())?;
        self.set_root(new_root);
        Ok(new_root)
    }

    pub fn root(&self) -> H256 {
        self.root
    }
    pub fn parent(&self) -> H256 {
        self.parent
    }
    pub fn values(&self) -> Result<StorageBackendSnapshot> {
        self.values.snapshot()
    }
    pub fn nodes(&self) -> Result<StorageBackendSnapshot> {
        self.nodes.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use crate::smt::SparseMerkleTree;
    use crate::CopyStrategy;
    use alloc::vec;
    use codec::{Decodable, Encodable};

    #[test]
    fn basic_get_set_check_root_test() {
        let mut smt = SparseMerkleTree::new();
        smt.update([1, 2, 3], [1, 2, 3]).unwrap();
        let root1 = smt.root;
        smt.update([4, 5, 3], [5, 2, 3]).unwrap();
        let root2 = smt.root;
        assert_ne!(root1, root2);
        let mut smt_2 = smt.subtree(CopyStrategy::Partial, vec![]).unwrap();
        assert_eq!(root2, smt_2.root);
        smt_2.update([1, 2, 3], [10, 20, 30]).unwrap();
        smt.update([1, 2, 3], [10, 20, 30]).unwrap();
        assert_eq!(smt.root, smt_2.root);

        let encoded = smt.encode().unwrap();

        let smt_decoded: SparseMerkleTree = SparseMerkleTree::decode(&encoded).unwrap();

        assert_eq!(
            smt.get([1, 2, 3]).unwrap(),
            smt_decoded.get([1, 2, 3]).unwrap()
        )
    }
}
