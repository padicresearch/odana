use std::sync::Arc;
use rocksdb::{DB};
use anyhow::{bail, Result};
use dashmap::DashMap;
use crate::error::Error;
use crate::store::{DatabaseBackend};


pub(crate) fn default_db_opts() -> rocksdb::Options {
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts.set_atomic_flush(true);

    // TODO: tune
    opts.increase_parallelism(num_cpus::get() as i32);
    opts.set_allow_mmap_writes(true);
    opts.set_allow_mmap_reads(true);

    opts.set_max_log_file_size(1_000_000);
    opts.set_recycle_log_file_num(5);
    opts.set_keep_log_file_num(5);
    opts
}

fn default_write_opts() -> rocksdb::WriteOptions {
    let mut opts = rocksdb::WriteOptions::default();
    opts.set_sync(true);
    opts
}

fn default_read_opts() -> rocksdb::ReadOptions {
    let mut opts = rocksdb::ReadOptions::default();
    opts
}


pub(crate) struct DiskStore {
    column_name: &'static str,
    inner: Arc<DB>,
}

impl DiskStore {
    pub(crate) fn new(column_name: &'static str, db: Arc<DB>) -> Self {
        Self {
            column_name,
            inner: db,
        }
    }
}

impl DatabaseBackend for DiskStore {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(self.column_name)
            .ok_or(Error::ColumnFamilyMissing(self.column_name))?;
        self.inner
            .put_cf_opt(&cf, key, value, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(self.column_name)
            .ok_or(Error::ColumnFamilyMissing(self.column_name))?;

        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        value.ok_or(Error::InvalidKey(key.to_vec()).into())
    }

    fn delete(&self, key: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(self.column_name)
            .ok_or(Error::ColumnFamilyMissing(self.column_name))?;

        self.inner
            .delete_cf_opt(&cf, key, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(self.column_name)
            .ok_or(Error::ColumnFamilyMissing(self.column_name))?;
        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        Ok(value.unwrap_or(default))
    }

}

#[derive(Debug, Clone)]
pub(crate) struct MemoryStore {
    inner: Arc<DashMap<Vec<u8>, Vec<u8>>>,
}

impl MemoryStore {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new())
        }
    }
}

impl DatabaseBackend for MemoryStore {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>
    {
        self.inner.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>
    {
        let value = self.inner.get(key).map(|r| r.value().clone());
        value.ok_or(Error::InvalidKey(key.to_vec()).into())
    }

    fn delete(&self, key: &[u8]) -> Result<()>
    {
        if !self.inner.contains_key(key) {
            bail!(Error::InvalidKey(key.to_vec()))
        }
        self.inner.remove(key);
        Ok(())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>
    {
        let value = self.inner.get(key).map(|r| r.value().clone());
        Ok(value.unwrap_or(default))
    }

}