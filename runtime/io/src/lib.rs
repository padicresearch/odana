#![no_std]
use anyhow::{bail, Result};
use odana_std::prelude::*;

mod internal {
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
}

pub trait StorageApi {
    fn set(&self, key: &[u8], value: &[u8]) {
        internal::storage::insert(key, value)
    }
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        internal::storage::get(key)
    }
    fn delete(&self, key: &[u8]) -> bool {
        internal::storage::remove(key)
    }
}

pub struct RawStorage;

impl StorageApi for RawStorage {}

pub trait StorageMap<K, V>
    where
        K: prost::Message + Default,
        V: prost::Message + Default,
{
    fn identifier() -> &'static [u8];

    fn insert(&self, key: K, value: V) {
        let identifier = Self::identifier();
        let key = key.encode_to_vec();
        let value = value.encode_to_vec();
        let storage_key = [identifier, key.as_slice()].concat();
        internal::storage::insert(storage_key.as_slice(), value.as_slice())
    }

    fn get(&self, key: K) -> Result<Option<V>> {
        let identifier = Self::identifier();
        let key = key.encode_to_vec();
        let storage_key = [identifier, key.as_slice()].concat();
        let Some(raw_value) = internal::storage::get(storage_key.as_slice()) else {
            return Ok(None)
        };
        let value = V::decode(raw_value.as_slice())?;
        Ok(Some(value))
    }

    fn remove(&self, key: K) -> Result<()> {
        let identifier = Self::identifier();
        let key = key.encode_to_vec();
        let storage_key = [identifier, key.as_slice()].concat();
        if !internal::storage::remove(storage_key.as_slice()) {
            bail!("failed to delete key")
        }
        Ok(())
    }
}
