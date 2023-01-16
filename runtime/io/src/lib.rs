#![no_std]

use anyhow::{bail, Result};
use core::marker::PhantomData;
use blake2b_simd::{blake2b, Params};
use rt_std::prelude::*;


pub struct Hashing;

impl Hashing {
    pub fn blake2b(input: &[u8]) -> [u8; 32] {
        let hash = Params::new()
            .hash_length(32)
            .hash(input);
        let mut out = [0_u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }
}


mod internal {
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
}

pub trait StorageApi {
    fn set(key: &[u8], value: &[u8]) {
        internal::storage::insert(key, value)
    }
    fn get(key: &[u8]) -> Option<Vec<u8>> {
        internal::storage::get(key)
    }
    fn delete(key: &[u8]) -> bool {
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
    fn storage_prefix() -> &'static [u8];

    fn prefix() -> [u8; 32] {
        let prefix = Self::storage_prefix();
        Hashing::blake2b(&prefix)
    }


    fn insert(key: K, value: V) {
        let prefix = Self::prefix();
        let key = key.encode_to_vec();
        let value = value.encode_to_vec();
        let storage_key = [prefix.as_slice(), key.as_slice()].concat();
        internal::storage::insert(storage_key.as_slice(), value.as_slice())
    }

    fn get(key: K) -> Result<Option<V>> {
        let prefix = Self::prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix.as_slice(), key.as_slice()].concat();
        let Some(raw_value) = internal::storage::get(storage_key.as_slice()) else {
            return Ok(None)
        };
        let value = V::decode(raw_value.as_slice())?;
        Ok(Some(value))
    }

    fn remove(key: K) -> Result<()> {
        let prefix = Self::prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix.as_slice(), key.as_slice()].concat();
        if !internal::storage::remove(storage_key.as_slice()) {
            bail!("failed to delete key")
        }
        Ok(())
    }
}