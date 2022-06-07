use codec::{Decoder, Encoder};

use crate::error::StorageError;
use crate::{KVStore, Schema, StorageIterator};

pub fn default_write_opts() -> rocksdb::WriteOptions {
    let mut opts = rocksdb::WriteOptions::default();
    opts.set_sync(true);
    opts
}

pub fn default_read_opts() -> rocksdb::ReadOptions {
    let mut opts = rocksdb::ReadOptions::default();
    opts
}

impl<S: Schema> KVStore<S> for rocksdb::DB {
    fn get(&self, key: &S::Key) -> anyhow::Result<Option<S::Value>> {
        let cf = self
            .cf_handle(S::column())
            .ok_or(StorageError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let value = self.get_cf(&cf, key)?;
        match value {
            None => Ok(None),
            Some(value) => Ok(Some(S::Value::decode(&value)?)),
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> anyhow::Result<()> {
        let cf = self
            .cf_handle(S::column())
            .ok_or(StorageError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let value = value.encode()?;
        self.put_cf_opt(&cf, key, value, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn delete(&self, key: &S::Key) -> anyhow::Result<()> {
        let cf = self
            .cf_handle(S::column())
            .ok_or(StorageError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        self.delete_cf(&cf, key).map_err(|e| e.into())
    }

    fn contains(&self, key: &S::Key) -> anyhow::Result<bool> {
        let cf = self
            .cf_handle(S::column())
            .ok_or(StorageError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let val = self.get_pinned_cf(&cf, key)?;
        Ok(val.is_some())
    }

    fn iter(&self) -> anyhow::Result<StorageIterator<S>> {
        let cf = self
            .cf_handle(S::column())
            .ok_or(StorageError::ColumnFamilyMissing(S::column()))?;
        let iter = self.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        Ok(Box::new(
            iter.map(|(k, v)| (S::Key::decode(&k), S::Value::decode(&v))),
        ))
    }
}
