mod internal {
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
}

use core::marker::PhantomData;
use anyhow::{bail, Result};
use blake2b_simd::Params;
use primitive_types::H256;
use sha3::Digest;
use xxhash_rust::const_xxh3::{xxh3_128, xxh3_64};

use rune_std::prelude::*;

pub trait StorageKeyHasher {
    fn hash(payload: &[u8]) -> Box<[u8]>;
}

pub struct Hashing;

impl Hashing {
    #[inline]
    pub fn blake2b(input: &[u8]) -> H256 {
        let hash = Params::new().hash_length(32).hash(input);
        let mut out = [0_u8; 32];
        out.copy_from_slice(hash.as_bytes());
        H256::from(out)
    }

    #[inline]
    pub fn keccak256(input: &[u8]) -> H256 {
        let mut hasher = sha3::Keccak256::default();
        hasher.update(input);
        let out = hasher.finalize();
        H256::from_slice(out.as_ref())
    }

    #[inline]
    pub fn twox_64_hash(payload: &[u8]) -> u64 {
        xxh3_64(payload)
    }

    #[inline]
    pub fn twox_128_hash(payload: &[u8]) -> u128 {
        xxh3_128(payload)
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

pub trait StorageKeyPrefix {
    fn key_prefix() -> &'static [u8];
}

pub struct StorageMap<Prefix, H: StorageKeyHasher, K: prost::Message + Default, V: prost::Message + Default> {
    inner : PhantomData<(Prefix,H, K, V)>
}

impl<Prefix, H, K, V> StorageMap<Prefix, H, K, V>
where
    Prefix: StorageKeyPrefix,
    H: StorageKeyHasher,
    K: prost::Message + Default,
    V: prost::Message + Default,
{
    pub fn put(key: K, value: V) {
        let prefix = Prefix::key_prefix();
        let key = key.encode_to_vec();
        let value = value.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        internal::storage::insert(&H::hash(storage_key.as_slice()), value.as_slice())
    }

    pub fn get(key: K) -> Result<Option<V>> {
        let prefix = Prefix::key_prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        let storage_key = H::hash(storage_key.as_slice());
        let Some(raw_value) = internal::storage::get(storage_key.as_ref()) else {
            return Ok(None)
        };
        if raw_value.is_empty() {
            return Ok(None);
        }
        let value = V::decode(raw_value.as_slice()).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(Some(value))
    }

    pub fn contains(key: K) -> bool {
        if let Ok(res) = Self::get(key) {
            return res.is_some();
        }
        false
    }

    fn remove(key: K) -> Result<()> {
        let prefix = Prefix::key_prefix();
        let key = key.encode_to_vec();
        let storage_key = [prefix, key.as_slice()].concat();
        if !internal::storage::remove(&H::hash(storage_key.as_slice())) {
            bail!("failed to delete key")
        }
        Ok(())
    }
}

pub struct StorageValue<Prefix, H: StorageKeyHasher, V: prost::Message + Default> {
    inner : PhantomData<(Prefix,H, V)>
}

impl<Prefix, H, V> StorageValue<Prefix, H, V>
    where
        Prefix: StorageKeyPrefix,
        H: StorageKeyHasher,
        V: prost::Message + Default,
{
    fn set(value: V) {
        let prefix = Prefix::key_prefix();
        let value = value.encode_to_vec();
        internal::storage::insert(&H::hash(prefix), value.as_slice())
    }

    fn get() -> Result<V> {
        let prefix = Prefix::key_prefix();
        let storage_key = H::hash(prefix);
        let Some(raw_value) = internal::storage::get(storage_key.as_ref()) else {
            bail!("value not found")
        };
        let value = V::decode(raw_value.as_slice()).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(value)
    }
}

pub struct Blake2bHasher;

impl StorageKeyHasher for Blake2bHasher {
    fn hash(payload: &[u8]) -> Box<[u8]> {
        Box::new(Hashing::blake2b(payload).to_fixed_bytes())
    }
}

pub struct Twox64Hasher;

impl StorageKeyHasher for Twox64Hasher {
    fn hash(payload: &[u8]) -> Box<[u8]> {
        Box::new(Hashing::twox_64_hash(payload).to_le_bytes())
    }
}

pub struct Twox128Hasher;

impl StorageKeyHasher for Twox128Hasher {
    fn hash(payload: &[u8]) -> Box<[u8]> {
        Box::new(Hashing::twox_128_hash(payload).to_le_bytes())
    }
}

pub fn emit<E: prost_extra::MessageExt + Default>(event: E) {
    emit_raw_event(E::full_name(), event.encode_to_vec().as_slice())
}

pub(crate) fn emit_raw_event(descriptor: &str, data: &[u8]) {
    internal::event::emit(descriptor, data);
}

pub fn print(msg: &str) {
    internal::logging::log(msg)
}
