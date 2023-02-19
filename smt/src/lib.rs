#![cfg_attr(not(test), no_std)]
#![feature(error_in_core)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::error::Error;
use crate::treehasher::TreeHasher;
use bincode::{Decode, Encode};
use primitive_types::H256;
pub use smt::*;

mod constants;
pub mod error;
pub mod proof;
pub mod smt;
pub mod treehasher;
pub mod utils;

#[derive(Copy, Clone)]
pub enum CopyStrategy {
    Partial,
    Full,
    None,
}

pub(crate) type Result<T> = core::result::Result<T, Error>;

pub type StorageBackendSnapshot = Vec<(Vec<u8>, Vec<u8>)>;

pub trait StorageBackend: Clone {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn get(&self, key: &[u8]) -> Result<Vec<u8>>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>;
    fn snapshot(&self) -> Result<StorageBackendSnapshot>;
    fn from_snapshot(snapshot: StorageBackendSnapshot) -> Result<Self>;
    fn new() -> Self;
}

#[derive(Clone)]
pub struct MemoryStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl StorageBackend for MemoryStorage {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let value = self.data.get(key).cloned();
        value.ok_or(Error::StorageErrorKeyNotFound)
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        if !self.data.contains_key(key) {
            return Err(Error::StorageError);
        }
        self.data.remove(key);
        Ok(())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>> {
        let value = self.data.get(key).cloned();
        Ok(value.unwrap_or(default))
    }

    fn snapshot(&self) -> Result<StorageBackendSnapshot> {
        let data = self.data.clone();
        let mut export = Vec::new();
        for (k, v) in data {
            export.push((k.clone(), v.clone()))
        }
        Ok(export)
    }

    fn from_snapshot(snapshot: StorageBackendSnapshot) -> Result<Self> {
        let mut data = BTreeMap::new();
        for (k, v) in snapshot {
            data.insert(k, v);
        }
        Ok(Self { data })
    }

    fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct DefaultTreeHasher;

impl TreeHasher for DefaultTreeHasher {
    fn digest(&self, data: &[u8]) -> H256 {
        crypto::keccak256(data)
    }
}
