use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use bincode::{Decode, Encode};
use codec::{Codec, Decodable, Encodable};
use primitive_types::H256;
use tracing::{debug, error};

use crate::error::StateError as Error;
use crate::store::Database;
use smt::proof::{verify_proof_with_updates, Proof};
use smt::treehasher::TreeHasher;
use smt::{CopyStrategy, DefaultTreeHasher, MemoryStorage, SparseMerkleTree, StorageBackend};

pub struct Options {
    strategy: CopyStrategy,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            strategy: CopyStrategy::Partial,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Encode, Decode)]
pub enum IValue {
    Deleted,
    Value(Vec<u8>),
}

impl Encodable for IValue {
    fn encode(&self) -> Result<Vec<u8>> {
        bincode::encode_to_vec(self, codec::config()).map_err(|e| e.into())
    }
}

impl Decodable for IValue {
    fn decode(buf: &[u8]) -> Result<Self> {
        bincode::decode_from_slice(buf, codec::config())
            .map(|(output, _)| output)
            .map_err(|e| e.into())
    }
}

pub enum Op<K: Codec, V: Codec> {
    Delete(K),
    Put(K, V),
}

pub struct Tree<K, V, H = DefaultTreeHasher> {
    db: Arc<Database>,
    head: Arc<RwLock<SparseMerkleTree<MemoryStorage, H>>>,
    staging: Arc<RwLock<SparseMerkleTree<MemoryStorage, H>>>,
    options: Options,
    hasher: H,
    _data: PhantomData<(K, V)>,
}

impl<K, V> Tree<K, V, DefaultTreeHasher>
where
    K: Codec,
    V: Codec,
{
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::open(path)?;
        let tree = match db.load_root() {
            Ok(tree) => tree,
            Err(_) => SparseMerkleTree::new(),
        };

        let options = Options::default();
        let staging_tree = tree.subtree(options.strategy, vec![])?;
        Ok(Self {
            db: Arc::new(db),
            head: Arc::new(RwLock::new(tree)),
            staging: Arc::new(RwLock::new(staging_tree)),
            options,
            hasher: DefaultTreeHasher,
            _data: Default::default(),
        })
    }

    pub fn open_read_only_at_root<P: AsRef<Path>>(path: P, root: &H256) -> Result<Self> {
        let db = Database::open_read_only(path)?;
        let tree = db.get(root)?;
        let options = Options::default();
        // let staging_tree = tree.subtree(options.strategy, vec![])?;
        Ok(Self {
            db: Arc::new(db),
            head: Arc::new(RwLock::new(tree.clone())),
            staging: Arc::new(RwLock::new(tree)),
            options,
            hasher: DefaultTreeHasher,
            _data: Default::default(),
        })
    }

    pub fn in_memory<P: AsRef<Path>>(options: Options) -> Result<Self> {
        let db = Database::in_memory();
        let tree = match db.load_root() {
            Ok(tree) => tree,
            Err(_) => SparseMerkleTree::new(),
        };

        let staging_tree = tree.subtree(options.strategy, vec![])?;
        Ok(Self {
            db: Arc::new(db),
            head: Arc::new(RwLock::new(tree)),
            staging: Arc::new(RwLock::new(staging_tree)),
            options,
            hasher: DefaultTreeHasher,
            _data: Default::default(),
        })
    }
}

impl<K, V, H> Tree<K, V, H>
    where
        K: Codec,
        V: Codec,
        H: TreeHasher,
{
    pub fn open_with_options<P: AsRef<Path>>(hasher: H, path: P, options: Options) -> Result<Self> {
        let db = Database::open(path)?;
        let tree = match db.load_root() {
            Ok(tree) => tree,
            Err(_) => SparseMerkleTree::new_with_hasher(
                hasher.clone(),
                MemoryStorage::new(),
                MemoryStorage::new(),
            ),
        };

        let staging_tree = tree.subtree(options.strategy, vec![])?;
        Ok(Self {
            db: Arc::new(db),
            head: Arc::new(RwLock::new(tree)),
            staging: Arc::new(RwLock::new(staging_tree)),
            options,
            hasher,
            _data: Default::default(),
        })
    }

    pub fn revert(&self, _root: H256) {
        todo!()
    }

    pub fn reset(&self, root: H256) -> Result<()> {
        let mut head = self.head.write().map_err(|_e| Error::RWPoison)?;
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;
        let new_head = self.db.get(&root)?;
        *head = new_head;
        *staging = head.subtree(self.options.strategy, vec![])?;
        Ok(())
    }

    pub fn rollback(&self) -> Result<()> {
        let head = self.head.write().map_err(|_e| Error::RWPoison)?;
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;
        *staging = head.subtree(self.options.strategy, vec![])?;
        Ok(())
    }

    pub fn head(&self) -> Result<SparseMerkleTree<MemoryStorage, H>> {
        let head = self.head.read().map_err(|_e| Error::RWPoison)?;
        Ok(head.clone())
    }

    pub fn apply(&self, batch: Vec<Op<K, V>>, persist: bool) -> Result<H256> {
        self.commit(persist)?;
        let res: Result<HashMap<_, _>> = batch
            .into_iter()
            .map(|op| match op {
                Op::Delete(key) => Ok((key.encode()?, Encodable::encode(&IValue::Deleted)?)),
                Op::Put(key, value) => Ok((
                    key.encode()?,
                    Encodable::encode(&IValue::Value(value.encode()?))?,
                )),
            })
            .collect();

        let batch = res?;

        let mut head = self.head.write().map_err(|_e| Error::RWPoison)?;
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;

        for (key, value) in batch {
            head.update(key, value)?;
        }

        let new_root = head.root();
        self.db.put(new_root, head.clone())?;
        *staging = head.subtree(self.options.strategy, vec![])?;
        Ok(new_root)
    }

    pub fn apply_non_commit(&self, at_root: &H256, batch: Vec<Op<K, V>>) -> Result<H256> {
        let mut tree: SparseMerkleTree<MemoryStorage, H> = self.db.get(at_root)?;
        let res: Result<HashMap<_, _>> = batch
            .into_iter()
            .map(|op| match op {
                Op::Delete(key) => Ok((key.encode()?, Encodable::encode(&IValue::Deleted)?)),
                Op::Put(key, value) => Ok((
                    key.encode()?,
                    Encodable::encode(&IValue::Value(value.encode()?))?,
                )),
            })
            .collect();

        let batch = res?;
        for (key, value) in batch {
            tree.update(key, value)?;
        }
        let new_root = tree.root();
        self.db.put(new_root, tree)?;
        Ok(new_root)
    }

    pub fn commit(&self, persist: bool) -> Result<H256> {
        let mut head = self.head.write().map_err(|_e| Error::RWPoison)?;
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;

        if head.root() == staging.root() {
            println!("head.root == staging.root");
            return Ok(head.root());
        }
        if persist {
            match self.db.put(staging.root(), staging.clone()) {
                Ok(_) => {
                    debug!(state_root = ?staging.root(), "Persisted State");
                }
                Err(e) => {
                    error!(error = ?e, "Unable to persisted");
                }
            }
        }
        *head = staging.clone();
        *staging = head.subtree(self.options.strategy, vec![])?;
        Ok(head.root())
    }

    pub fn put(&self, key: K, value: V) -> Result<()> {
        let (key, value) = (
            key.encode()?,
            Encodable::encode(&IValue::Value(value.encode()?))?,
        );
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;
        let _root = staging.update(key, value)?;
        Ok(())
    }

    pub fn delete(&self, key: &K) -> Result<()> {
        let (key, value) = (key.encode()?, Encodable::encode(&IValue::Deleted)?);
        let mut staging = self.staging.write().map_err(|_e| Error::RWPoison)?;
        staging.update(key, value)?;
        Ok(())
    }

    pub fn get(&self, key: &K) -> Result<Option<V>> {
        let descend = match self.options.strategy {
            CopyStrategy::Partial => true,
            CopyStrategy::Full => false,
            CopyStrategy::None => false,
        };

        self.get_descend(key, descend)
    }

    pub fn get_at_root(&self, from_root: &H256, key: &K) -> Result<Option<V>> {
        let descend = match self.options.strategy {
            CopyStrategy::Partial => true,
            CopyStrategy::Full => false,
            CopyStrategy::None => false,
        };

        self.get_descend_from_root(from_root, key, descend)
    }

    pub fn get_with_proof(&self, key: &K) -> Result<(V, Proof)> {
        let head = self.head.read().map_err(|_e| Error::RWPoison)?;
        let raw_key = key.encode()?;
        let value = self
            .get(key)?
            .ok_or_else(|| Error::InvalidKey(hex::encode(raw_key.as_slice(), false)))?;
        let proof = head.proof(&raw_key)?;
        Ok((value, proof))
    }

    pub fn get_descend(&self, key: &K, descend: bool) -> Result<Option<V>> {
        let key = key.encode()?;
        let staging = self.staging.read().map_err(|_e| Error::RWPoison)?;
        let head = self.head.read().map_err(|_e| Error::RWPoison)?;
        let mut value = staging.get(&key)?;
        if value.is_empty() && descend {
            let res = self._get_descend(&key, &head.root())?;
            match res {
                None => return Ok(None),
                Some(encoded_value) => {
                    value = encoded_value;
                }
            }
        } else if value.is_empty() && !descend {
            value = head.get(&key)?;
        }

        if value.is_empty() {
            return Ok(None);
        }
        let decoded_value = <IValue as Decodable>::decode(&value)?;
        match decoded_value {
            IValue::Deleted => Ok(None),
            IValue::Value(value) => Ok(Some(V::decode(&value)?)),
        }
    }

    fn get_descend_from_root(&self, from_root: &H256, key: &K, descend: bool) -> Result<Option<V>> {
        let key = key.encode()?;
        let head: SparseMerkleTree<MemoryStorage, H> = self.db.get(from_root)?;
        let mut value = head.get(&key)?;
        if value.is_empty() && descend {
            let res = self._get_descend(&key, &head.root())?;
            match res {
                None => return Ok(None),
                Some(encoded_value) => {
                    value = encoded_value;
                }
            }
        } else if value.is_empty() && !descend {
            value = head.get(&key)?;
        }

        if value.is_empty() {
            return Ok(None);
        }
        let decoded_value = <IValue as Decodable>::decode(&value)?;
        match decoded_value {
            IValue::Deleted => Ok(None),
            IValue::Value(value) => Ok(Some(V::decode(&value)?)),
        }
    }

    fn _get_descend(&self, key: &[u8], root: &H256) -> Result<Option<Vec<u8>>> {
        let mut root = *root;
        loop {
            let tree: SparseMerkleTree<MemoryStorage, H> = self.db.get(&root)?;
            let value = tree.get(key)?;
            if value.is_empty() && tree.root() != tree.parent() {
                root = tree.parent();
                continue;
            } else if value.is_empty() && tree.root() == tree.parent() {
                return Ok(None);
            }
            return Ok(Some(value));
        }
    }

    pub fn root(&self) -> Result<H256> {
        let head = self.head.write().map_err(|_e| Error::RWPoison)?;
        Ok(head.root())
    }

    pub fn hasher(&self) -> &H {
        &self.hasher
    }
}

pub struct Verifier;

impl Verifier {
    pub fn verify_proof<K, V, H>(
        hasher: &H,
        proof: &Proof,
        root: H256,
        key: K,
        value: V,
    ) -> Result<()>
        where
            K: Codec,
            V: Codec,
            H: TreeHasher,
    {
        let key = key.encode()?;
        let value = Encodable::encode(&IValue::Value(value.encode()?))?;
        verify_proof_with_updates(hasher, proof, root, &key, &value)
            .map(|_| ())
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use crate::tree::{Tree, Verifier};
    use primitive_types::{H160, H256};
    use types::account::AccountState;

    #[test]
    fn basic_test() {
        let tmp_dir = TempDir::new("test").unwrap();
        let tree = Tree::open(tmp_dir.path()).unwrap();
        tree.put(
            H256::from_slice(&[1; 32]),
            AccountState {
                free_balance: 30000,
                reserve_balance: 3000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[2; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[3; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[24; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();
        let root_1 = tree.commit(true).unwrap();
        println!("ROOT {:?}", root_1);
        tree.put(
            H256::from_slice(&[24; 32]),
            AccountState {
                free_balance: 20000,
                reserve_balance: 2000,
                nonce: 2,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[44; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[32; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[50; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
        .unwrap();

        tree.put(
            H256::from_slice(&[3; 32]),
            AccountState {
                free_balance: 200,
                reserve_balance: 200,
                nonce: 3,
            },
        )
        .unwrap();

        let root_2 = tree.commit(true).unwrap();
        println!("ROOT {:?}", root_2);

        assert_eq!(
            tree.get_descend(&H256::from_slice(&[3; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 200,
                reserve_balance: 200,
                nonce: 3,
            })
        );
        tree.reset(root_1).unwrap();

        assert_eq!(
            tree.get_descend(&H256::from_slice(&[3; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            })
        );

        tree.reset(root_2).unwrap();

        tree.put(
            H256::from_slice(&[3; 32]),
            AccountState {
                free_balance: 90000,
                reserve_balance: 9000,
                nonce: 2,
            },
        )
        .unwrap();

        let _root_3 = tree.commit(true).unwrap();

        assert_eq!(
            tree.get_descend(&H256::from_slice(&[3; 32]), false)
                .unwrap(),
            Some(AccountState {
                free_balance: 90000,
                reserve_balance: 9000,
                nonce: 2,
            })
        );

        assert_eq!(
            tree.get_descend(&H256::from_slice(&[1; 32]), false)
                .unwrap(),
            None
        );
        assert_eq!(
            tree.get_descend(&H256::from_slice(&[1; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 30000,
                reserve_balance: 3000,
                nonce: 1,
            },)
        );

        let (value, proof) = tree.get_with_proof(&H256::from_slice(&[1; 32])).unwrap();
        println!("{:#?}", (&value, &proof));
        println!(
            "{:?}",
            Verifier::verify_proof(
                tree.hasher(),
                &proof,
                tree.root().unwrap(),
                H256::from_slice(&[1; 32]),
                value,
            )
        )
    }

    #[test]
    fn basic_test_genesis_root() {
        let tmp_dir = TempDir::new("test").unwrap();
        let tree = Tree::open(tmp_dir.path()).unwrap();
        tree.put(
            H160::from_slice(&[0; 20]),
            AccountState {
                free_balance: 1_000_000_000,
                reserve_balance: 0,
                nonce: 1,
            },
        )
        .unwrap();
        tree.commit(true).unwrap();
        println!("{:#?}", tree.root().unwrap());
        println!("{:?}", tree.root().unwrap().0);
    }
}
