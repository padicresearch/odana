#![no_std]

use bincode::{Decode, Encode};
use anyhow::{bail, Result};
use odana_std::prelude::*;

mod internal {
    include!("../../generated/storage.rs");
}


pub trait StorageApi {
    fn set(&self, key : &[u8], value : &[u8])  {
        internal::storage::insert(key, value)
    }
    fn get(&self, key : &[u8]) -> Option<Vec<u8>> {
        internal::storage::get(key)
    }

    fn delete(&self, key : &[u8]) -> bool {
        internal::storage::remove(key)
    }

    fn root(&self) -> Vec<u8> {
        internal::storage::root()
    }
}

pub struct RawStorage;

impl StorageApi for RawStorage {}


pub trait StorageMap<K,V> where  K : Encode + Decode, V : Encode + Decode {
    fn identifier() -> &'static [u8];

    fn insert(&self, key : K, value : V) -> Result<()> {
        let identifier = Self::identifier();
        let key = bincode::encode_to_vec(key, bincode::config::standard())?;
        let value = bincode::encode_to_vec(value, bincode::config::standard())?;
        let storage_key = [identifier, key.as_slice()].concat();
        Ok(internal::storage::insert(storage_key.as_slice(), value.as_slice()))
    }


    fn get(&self, key : K) -> Result<Option<Vec<u8>>> {
        let identifier = Self::identifier();
        let key = bincode::encode_to_vec(key, bincode::config::standard())?;
        let storage_key = [identifier, key.as_slice()].concat();
        Ok(internal::storage::get(storage_key.as_slice()))
    }


    fn remove(&self, key : K) -> Result<()> {
        let identifier = Self::identifier();
        let key = bincode::encode_to_vec(key, bincode::config::standard())?;
        let storage_key = [identifier, key.as_slice()].concat();
        if !internal::storage::remove(storage_key.as_slice()) {
            bail!("failed to delete key")
        }
        Ok(())
    }

}