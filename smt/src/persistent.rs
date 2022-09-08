use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Result};
use dashmap::DashMap;
use rocksdb::checkpoint::Checkpoint;
use rocksdb::DB;

use crate::error::Error;
use crate::store::{cfs, DatabaseBackend};

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
    rocksdb::ReadOptions::default()
}

pub(crate) struct RocksDB {
    inner: Arc<DB>,
}

impl RocksDB {
    pub(crate) fn new(db: Arc<DB>) -> Self {
        Self { inner: db }
    }
}

impl DatabaseBackend for RocksDB {
    fn put(&self, column_name: &'static str, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self
            .inner
            .cf_handle(column_name)
            .ok_or(Error::ColumnFamilyMissing(column_name))?;
        self.inner
            .put_cf_opt(&cf, key, value, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get(&self, column_name: &'static str, key: &[u8]) -> Result<Vec<u8>> {
        let cf = self
            .inner
            .cf_handle(column_name)
            .ok_or(Error::ColumnFamilyMissing(column_name))?;

        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        value.ok_or_else(|| Error::InvalidKey(hex::encode(key, false)).into())
    }

    fn delete(&self, column_name: &'static str, key: &[u8]) -> Result<()> {
        let cf = self
            .inner
            .cf_handle(column_name)
            .ok_or(Error::ColumnFamilyMissing(column_name))?;

        self.inner
            .delete_cf_opt(&cf, key, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn checkpoint(&self, path: PathBuf) -> Result<Arc<dyn DatabaseBackend + Send + Sync>> {
        Checkpoint::new(self.inner.as_ref())?.create_checkpoint(path.as_path())?;
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(
            &default_db_opts(),
            path.as_path(),
            cfs(),
        )?);
        Ok(Arc::new(RocksDB::new(db)))
    }

    fn get_or_default(
        &self,
        column_name: &'static str,
        key: &[u8],
        default: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let cf = self
            .inner
            .cf_handle(column_name)
            .ok_or(Error::ColumnFamilyMissing(column_name))?;
        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        Ok(value.unwrap_or(default))
    }
}

type Column = DashMap<Vec<u8>, Vec<u8>>;

#[derive(Debug, Clone)]
pub(crate) struct MemoryStore {
    inner: Arc<DashMap<&'static str, Column>>,
}

impl MemoryStore {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }
}

impl DatabaseBackend for MemoryStore {
    fn put(&self, column_name: &'static str, key: &[u8], value: &[u8]) -> Result<()> {
        let column = self.inner.entry(column_name).or_default();
        column.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, column_name: &'static str, key: &[u8]) -> Result<Vec<u8>> {
        let column = self.inner.entry(column_name).or_default();
        let value = column.get(key).map(|r| r.value().clone());
        value.ok_or_else(|| Error::InvalidKey(hex::encode(key, false)).into())
    }

    fn delete(&self, column_name: &'static str, key: &[u8]) -> Result<()> {
        let column = self.inner.entry(column_name).or_default();
        if !column.contains_key(key) {
            bail!(Error::InvalidKey(hex::encode(key, false)))
        }
        column.remove(key);
        Ok(())
    }

    fn checkpoint(&self, _: PathBuf) -> Result<Arc<dyn DatabaseBackend + Send + Sync>> {
        Ok(Arc::new(MemoryStore {
            inner: self.inner.clone(),
        }))
    }

    fn get_or_default(
        &self,
        column_name: &'static str,
        key: &[u8],
        default: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let column = self.inner.entry(column_name).or_default();
        let value = column.get(key).map(|r| r.value().clone());
        Ok(value.unwrap_or(default))
    }
}
