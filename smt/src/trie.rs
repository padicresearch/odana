use crate::store::Database;
use crate::SparseMerkleTree;
use anyhow::{Result};
use codec::{Codec, Decoder, Encoder};
use primitive_types::H256;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Clone)]
pub enum IValue {
    Deleted,
    Value(Vec<u8>),
}

impl Encoder for IValue {}

impl Decoder for IValue {}

pub enum Op<K: Codec, V: Codec> {
    Delete(K),
    Put(K, V),
}

pub struct Trie<K, V> {
    db: Arc<Database>,
    head: Arc<RwLock<SparseMerkleTree>>,
    staging: Arc<RwLock<SparseMerkleTree>>,
    staging_keys: Arc<RwLock<Vec<Vec<u8>>>>,
    _data: PhantomData<(K, V)>,
}

impl<K, V> Trie<K, V>
    where
        K: Codec,
        V: Codec,
{
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Database::open(path)?;
        let tree = match db.load_root() {
            Ok(tree) => tree,
            Err(_) => SparseMerkleTree::new(),
        };

        let staging_tree = tree.subtree(true, vec![])?;
        Ok(Self {
            db: Arc::new(db),
            head: Arc::new(RwLock::new(tree.clone())),
            staging: Arc::new(RwLock::new(staging_tree)),
            staging_keys: Default::default(),
            _data: Default::default(),
        })
    }

    pub fn revert(&self, root: H256) {
        todo!()
    }

    pub fn reset(&self, root: H256) -> Result<()> {
        let mut head = self.head.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut staging = self.staging.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut staging_keys = self
            .staging_keys
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let new_head = self.db.get(&root)?;
        *head = new_head;
        *staging = head.subtree(true, vec![])?;
        staging_keys.clear();
        Ok(())
    }

    pub fn head(&self) -> Result<SparseMerkleTree> {
        let mut head = self.head.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(head.clone())
    }

    pub fn apply(&self, batch: Vec<Op<K, V>>) -> Result<H256> {
        self.commit()?;
        let res: Result<HashMap<_, _>> = batch
            .into_iter()
            .map(|op| match op {
                Op::Delete(key) => Ok((key.encode()?, IValue::Deleted.encode()?)),
                Op::Put(key, value) => {
                    Ok((key.encode()?, IValue::Value(value.encode()?).encode()?))
                }
            })
            .collect();

        let batch = res?;

        let mut head = self.head.write().map_err(|e| anyhow::anyhow!("{}", e))?;

        for (key, value) in batch {
            head.update(key, value)?;
        }

        let new_root = head.root();
        self.db.put(new_root, head.clone());
        Ok(new_root)
    }

    pub fn commit(&self) -> Result<H256> {
        let mut head = self.head.write().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut staging = self.staging.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut staging_keys = self
            .staging_keys
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        self.db.put(staging.root(), staging.clone());
        *head = staging.clone();
        *staging = head.subtree(true, vec![])?;
        staging_keys.clear();
        Ok(head.root())
    }

    pub fn put(&self, key: K, value: V) -> Result<()> {
        let (key, value) = (key.encode()?, IValue::Value(value.encode()?).encode()?);
        let mut staging = self.staging.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut staging_keys = self
            .staging_keys
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        staging.update(key.clone(), value);
        staging_keys.push(key);
        Ok(())
    }

    pub fn delete(&self, key: &K) -> Result<()> {
        let (key, value) = (key.encode()?, IValue::Deleted.encode()?);
        let mut staging = self.staging.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut staging_keys = self
            .staging_keys
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        staging.update(key.clone(), value);
        staging_keys.push(key);
        Ok(())
    }

    pub fn get(&self, key: &K, descend: bool) -> Result<Option<V>> {
        let key = key.encode()?;
        let mut staging = self.staging.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut head = self.head.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut value = staging.get(&key)?;
        if value.is_empty() && descend {
            let res = self.get_descend(&key, &head.root)?;
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
        let decoded_value = IValue::decode(&value)?;
        return match decoded_value {
            IValue::Deleted => Ok(None),
            IValue::Value(value) => Ok(Some(V::decode(&value)?)),
        };
    }

    fn get_descend(&self, key: &[u8], root: &H256) -> Result<Option<Vec<u8>>> {
        let mut root = *root;
        loop {
            let tree = self.db.get(&root)?;
            let value = tree.get(key)?;
            if value.is_empty() && tree.root != tree.parent {
                root = tree.parent;
                continue;
            } else if value.is_empty() && tree.root == tree.parent {
                return Ok(None);
            }
            return Ok(Some(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Trie;
    use primitive_types::H256;
    use tempdir::TempDir;
    use types::account::AccountState;

    #[test]
    fn basic_test() {
        let tmp_dir = TempDir::new("test").unwrap();
        let trie = Trie::open(tmp_dir.path()).unwrap();
        trie.put(
            H256::from_slice(&vec![1; 32]),
            AccountState {
                free_balance: 30000,
                reserve_balance: 3000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![2; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![3; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![24; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();
        let root_1 = trie.commit().unwrap();
        trie.put(
            H256::from_slice(&vec![24; 32]),
            AccountState {
                free_balance: 20000,
                reserve_balance: 2000,
                nonce: 2,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![44; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![32; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![50; 32]),
            AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            },
        )
            .unwrap();

        trie.put(
            H256::from_slice(&vec![3; 32]),
            AccountState {
                free_balance: 200,
                reserve_balance: 200,
                nonce: 3,
            },
        )
            .unwrap();

        let root_2 = trie.commit().unwrap();

        assert_eq!(
            trie.get(&H256::from_slice(&vec![3; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 200,
                reserve_balance: 200,
                nonce: 3,
            })
        );
        trie.reset(root_1).unwrap();

        assert_eq!(
            trie.get(&H256::from_slice(&vec![3; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 10000,
                reserve_balance: 1000,
                nonce: 1,
            })
        );

        trie.reset(root_2).unwrap();

        trie.put(
            H256::from_slice(&vec![3; 32]),
            AccountState {
                free_balance: 90000,
                reserve_balance: 9000,
                nonce: 2,
            },
        )
            .unwrap();

        let root_3 = trie.commit().unwrap();

        assert_eq!(
            trie.get(&H256::from_slice(&vec![3; 32]), false).unwrap(),
            Some(AccountState {
                free_balance: 90000,
                reserve_balance: 9000,
                nonce: 2,
            })
        );

        assert_eq!(
            trie.get(&H256::from_slice(&vec![1; 32]), false).unwrap(),
            None
        );
        assert_eq!(
            trie.get(&H256::from_slice(&vec![1; 32]), true).unwrap(),
            Some(AccountState {
                free_balance: 30000,
                reserve_balance: 3000,
                nonce: 1,
            }, )
        );
    }
}
