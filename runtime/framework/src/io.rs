mod internal {
    use rune_std::prelude::*;
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
}

use anyhow::{bail, Result};
use blake2b_simd::{blake2b, Params};
use core::marker::PhantomData;
use rune_std::prelude::*;

pub trait StorageKeyHasher {
    fn hash(payload: &[u8]) -> Box<[u8]>;
}

pub struct Hashing;

impl Hashing {
    pub fn blake2b(input: &[u8]) -> [u8; 32] {
        let hash = Params::new().hash_length(32).hash(input);
        let mut out = [0_u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }
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

pub trait StorageMap<H, K, V>
where
    H: StorageKeyHasher,
    K: prost::Message + Default,
    V: prost::Message + Default,
{
    fn storage_prefix() -> &'static [u8];

    fn insert(key: K, value: V) {
        let prefix = Self::storage_prefix();
        let key = key.encode_to_vec();
        let value = value.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        internal::storage::insert(&H::hash(storage_key.as_slice()), value.as_slice())
    }

    fn get(key: K) -> Result<Option<V>> {
        let prefix = Self::storage_prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        let storage_key = H::hash(storage_key.as_slice());
        let Some(raw_value) = internal::storage::get(storage_key.as_ref()) else {
            return Ok(None)
        };
        let value = V::decode(raw_value.as_slice())?;
        Ok(Some(value))
    }

    fn remove(key: K) -> Result<()> {
        let prefix = Self::storage_prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        if !internal::storage::remove(&H::hash(storage_key.as_slice())) {
            bail!("failed to delete key")
        }
        Ok(())
    }
}

pub trait StorageValue<H, V>
where
    H: StorageKeyHasher,
    V: prost::Message + Default,
{
    fn storage_prefix() -> &'static [u8];
    fn storage_key() -> &'static [u8];

    fn set(value: V) {
        let prefix = Self::storage_prefix();
        let value = value.encode_to_vec();
        let storage_key = [prefix, Self::storage_key()].concat();
        internal::storage::insert(&H::hash(storage_key.as_slice()), value.as_slice())
    }

    fn get() -> Result<V> {
        let prefix = Self::storage_prefix();
        let storage_key = [prefix, Self::storage_key()].concat();
        let storage_key = H::hash(storage_key.as_slice());
        let Some(raw_value) = internal::storage::get(storage_key.as_ref()) else {
            bail!("value not found")
        };
        let value = V::decode(raw_value.as_slice())?;
        Ok(value)
    }
}

pub struct Blake2bHasher;

impl StorageKeyHasher for Blake2bHasher {
    fn hash(payload: &[u8]) -> Box<[u8]> {
        Box::new(Hashing::blake2b(payload))
    }
}

pub fn emit<E: prost::Message + Default>(event: E) {
    internal::event::emit(&event.encode_to_vec())
}
