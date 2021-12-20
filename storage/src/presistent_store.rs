use crate::{Storage, KVEntry, StorageIterator};
use sled::IVec;
use crate::codec::{Encoder, Decoder};
use itertools::Itertools;
use anyhow::Result;
use std::path::Path;

pub struct PersistentStore {
    inner: sled::Db,
}

impl PersistentStore {
    pub fn new<P : AsRef<Path>>(path : P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self {
            inner: db
        })
    }
}

impl<S: KVEntry> Storage<S> for PersistentStore {
    fn get(&self, key: &S::Key) -> anyhow::Result<Option<S::Value>> {
        let key = key.encode()?;
        let result = self.inner.get(&key)?;
        match result {
            None => {
                Ok(None)
            }
            Some(raw) => {
                Ok(Some(S::Value::decode(raw.as_ref())?))
            }
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> anyhow::Result<()> {
        let key = key.encode()?;
        let value = value.encode()?;
        self.inner.insert(key, value)?;
        Ok(())
    }

    fn delete(&self, key: &S::Key) -> anyhow::Result<()> {
        let key = key.encode()?;
        self.inner.remove(key)?;
        Ok(())
    }

    fn contains(&self, key: &S::Key) -> anyhow::Result<bool> {
        let key = key.encode()?;
        self.inner.contains_key(key).map_err(|e| e.into())
    }

    fn iter(&self) -> Result<StorageIterator<S>> {
        let iter = self.inner.iter();
        Ok(Box::new(iter.map(|result| {
            let (k,v) = result.unwrap();
            (S::Key::decode(k.as_ref()), S::Value::decode(v.as_ref()))
        })))
    }
}