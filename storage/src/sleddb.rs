use std::path::Path;

use anyhow::Result;
use sled::Tree;

use codec::{Decodable, Encodable};

use crate::{KVStore, Schema, StorageIterator};

pub struct SledDB {
    inner: sled::Db,
}

impl SledDB {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { inner: db })
    }

    fn column(&self, name: &'static str) -> Result<Tree> {
        self.inner.open_tree(name).map_err(|e| e.into())
    }
}

impl<S: Schema> KVStore<S> for SledDB {
    fn get(&self, key: &S::Key) -> anyhow::Result<Option<S::Value>> {
        let key = key.encode()?;
        let result = self.column(S::column())?.get(key)?;
        match result {
            None => Ok(None),
            Some(raw) => Ok(Some(S::Value::decode(raw.as_ref())?)),
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> anyhow::Result<()> {
        let key = key.encode()?;
        let value = value.encode()?;
        self.column(S::column())?.insert(key, value)?;
        Ok(())
    }

    fn delete(&self, key: &S::Key) -> anyhow::Result<()> {
        let key = key.encode()?;
        self.column(S::column())?.remove(key)?;
        Ok(())
    }

    fn contains(&self, key: &S::Key) -> anyhow::Result<bool> {
        let key = key.encode()?;
        self.column(S::column())?
            .contains_key(key)
            .map_err(|e| e.into())
    }

    fn iter(&self) -> Result<StorageIterator<S>> {
        let iter = self.column(S::column())?.iter();
        Ok(Box::new(iter.map(|result| {
            let (k, v) = result.unwrap();
            (S::Key::decode(k.as_ref()), S::Value::decode(v.as_ref()))
        })))
    }

    fn prefix_iter(&self, start: &S::Key) -> Result<StorageIterator<S>> {
        let start = start.encode()?;
        let iter = self.column(S::column())?.scan_prefix(start);
        Ok(Box::new(iter.map(|result| {
            let (k, v) = result.unwrap();
            (S::Key::decode(k.as_ref()), S::Value::decode(v.as_ref()))
        })))
    }
}
