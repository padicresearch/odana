use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::{Error, Result};

use codec::{Codec, Decoder, Encoder};

use crate::error::StorageError;
use crate::{KVStore, StorageIterator};
use crate::Schema;

#[derive(Debug)]
pub struct ColumnMemStore {
    inner: Arc<RwLock<BTreeMap<Arc<Vec<u8>>, Arc<Vec<u8>>>>>,
}

impl Default for ColumnMemStore {
    fn default() -> Self {
        Self {
            inner: Arc::new(Default::default()),
        }
    }
}

#[derive(Debug)]
pub struct MemStore {
    inner: Arc<BTreeMap<&'static str, Arc<ColumnMemStore>>>,
}

pub struct MemStoreIterator {
    cursor: usize,
    inner: Vec<(Arc<Vec<u8>>, Arc<Vec<u8>>)>,
}

impl MemStoreIterator {
    fn new(store: Arc<RwLock<BTreeMap<Arc<Vec<u8>>, Arc<Vec<u8>>>>>) -> Self {
        let inner = store.clone();
        let store = inner.read().map_err(|_| StorageError::RWPoison).unwrap();
        Self {
            cursor: 0,
            inner: store.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        }
    }
}

impl Iterator for MemStoreIterator {
    type Item = (Arc<Vec<u8>>, Arc<Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor > self.inner.len() {
            return None;
        }

        let res = self
            .inner
            .get(self.cursor)
            .map(|(k, v)| (k.clone(), v.clone()));
        self.cursor += 1;
        res
    }
}

impl MemStore {
    pub fn new(columns: Vec<&'static str>) -> Self {
        let columns: BTreeMap<_, _> = columns
            .iter()
            .map(|name| (*name, Arc::new(ColumnMemStore::default())))
            .collect();
        Self {
            inner: Arc::new(columns),
        }
    }

    fn column(&self, name: &'static str) -> Result<Arc<ColumnMemStore>> {
        match self.inner.get(name) {
            None => {
                anyhow::bail!("")
            }
            Some(col) => Ok(col.clone()),
        }
    }
}

impl<S: Schema> KVStore<S> for MemStore {
    fn get(&self, key: &S::Key) -> Result<Option<S::Value>> {
        let key = key.encode()?;
        match self.column(S::column())?.get(key)? {
            None => Ok(None),
            Some(value) => Ok(Some(S::Value::decode(&value)?)),
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> Result<()> {
        let key = key.encode()?;
        let value = value.encode()?;
        self.column(S::column())?.put(key, value)
    }

    fn delete(&self, key: &S::Key) -> Result<()> {
        let key = key.encode()?;
        self.column(S::column())?.delete(key)
    }

    fn contains(&self, key: &S::Key) -> Result<bool> {
        let key = key.encode()?;
        self.column(S::column())?.contains(key)
    }

    fn iter<'a>(
        &'a self,
    ) -> Result<Box<dyn 'a + Send + Iterator<Item=(Result<S::Key>, Result<S::Value>)>>> {
        Ok(Box::new(
            self.column(S::column())?
                .iter()
                .map(|(k, v)| (S::Key::decode(&k), S::Value::decode(&v))),
        ))
    }

    fn prefix_iter(&self, start: &S::Key) -> Result<StorageIterator<S>> {
        todo!()
    }
}

impl ColumnMemStore {
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let inner = self.inner.clone();
        let store = inner.read().map_err(|_| StorageError::RWPoison)?;
        let result = store.get(&key);
        match result {
            Some(value) => Ok(Some(value.deref().clone())),
            None => Ok(None),
        }
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let inner = self.inner.clone();
        let mut store = inner.write().map_err(|_| StorageError::RWPoison)?;
        store.insert(Arc::new(key), Arc::new(value));
        Ok(())
    }

    fn delete(&self, key: Vec<u8>) -> Result<()> {
        let inner = self.inner.clone();
        let mut store = inner.write().map_err(|_| StorageError::RWPoison)?;
        store.remove(&key);
        Ok(())
    }

    fn contains(&self, key: Vec<u8>) -> Result<bool> {
        let inner = self.inner.clone();
        let store = inner.read().map_err(|_| StorageError::RWPoison)?;
        Ok(store.contains_key(&key))
    }

    fn iter(&self) -> MemStoreIterator {
        MemStoreIterator::new(self.inner.clone())
    }
}
