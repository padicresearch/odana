use crate::Storage;
use crate::KVEntry;
use crate::error::StorageError;
use crate::codec::{Codec, Encoder, Decoder};
use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;




#[derive(Debug)]
pub struct MemStore {
    inner: Arc<RwLock<BTreeMap<Arc<Vec<u8>>, Arc<Vec<u8>>>>>,
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
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Default::default()),
        }
    }
}

impl<S: KVEntry> Storage<S> for MemStore {
    fn get(&self, key: &S::Key) -> Result<Option<S::Value>> {
        let inner = self.inner.clone();
        let store = inner.read().map_err(|_| StorageError::RWPoison)?;
        let key = key.encode()?;
        let result = store.get(&key);
        match result {
            Some(value) => Ok(Some(S::Value::decode(value)?)),
            None => Ok(None),
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> Result<()> {
        let inner = self.inner.clone();
        let mut store = inner.write().map_err(|_| StorageError::RWPoison)?;
        let key = key.encode()?;
        let value = value.encode()?;
        store.insert(Arc::new(key), Arc::new(value));
        Ok(())
    }

    fn delete(&self, key: &S::Key) -> Result<()> {
        let inner = self.inner.clone();
        let mut store = inner.write().map_err(|_| StorageError::RWPoison)?;
        let key = key.encode()?;
        store.remove(&key);
        Ok(())
    }

    fn contains(&self, key: &S::Key) -> Result<bool> {
        let inner = self.inner.clone();
        let store = inner.read().map_err(|_| StorageError::RWPoison)?;
        let key = key.encode()?;
        Ok(store.contains_key(&key))
    }

    fn iter<'a>(
        &'a self,
    ) -> Result<Box<dyn 'a + Send + Iterator<Item = (Result<S::Key>, Result<S::Value>)>>> {
        let iter = MemStoreIterator::new(self.inner.clone());
        Ok(Box::new(
            iter.into_iter()
                .map(|(k, v)| (S::Key::decode(&k), S::Value::decode(&v))),
        ))
    }
}