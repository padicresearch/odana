use std::sync::Arc;
use rocksdb::{BlockBasedOptions, ColumnFamilyDescriptor, DB, Options};
use anyhow::Result;
use dashmap::DashMap;
use crate::store::{DatabaseBackend, StorageError};


pub(crate) fn default_db_opts() -> rocksdb::Options {
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts.set_atomic_flush(true);

    // TODO: tune
    opts.increase_parallelism(num_cpus::get() as i32);
    // opts.set_advise_random_on_open(false);
    opts.set_allow_mmap_writes(true);
    opts.set_allow_mmap_reads(true);

    opts.set_max_log_file_size(1_000_000);
    opts.set_recycle_log_file_num(5);
    opts.set_keep_log_file_num(5);
    //opts.selo

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

fn default_table_options() -> Options {
    // default db options
    let mut db_opts = Options::default();

    // https://github.com/facebook/rocksdb/wiki/Setup-Options-and-Basic-Tuning#other-general-options
    db_opts.set_level_compaction_dynamic_level_bytes(false);
    db_opts.set_write_buffer_size(32 * 1024 * 1024);

    // block table options
    let mut table_options = BlockBasedOptions::default();
    // table_options.set_block_cache(&Cache::new_lru_cache(32 * 1024 * 1024).unwrap());
    // table_options.set_block_size(16 * 1024);
    // table_options.set_cache_index_and_filter_blocks(true);
    // table_options.set_pin_l0_filter_and_index_blocks_in_cache(true);

    // set format_version 4 https://rocksdb.org/blog/2019/03/08/format-version-4.html
    table_options.set_format_version(4);
    table_options.set_index_block_restart_interval(16);

    db_opts.set_block_based_table_factory(&table_options);

    db_opts
}

pub(crate) fn cfs() -> Vec<ColumnFamilyDescriptor> {
    vec![
        ColumnFamilyDescriptor::new(NodeColumn::column_name(), default_table_options()),
        ColumnFamilyDescriptor::new(ValueColumn::column_name(), default_table_options()),
    ]
}

pub(crate) struct NodeColumn {
    inner: Arc<DB>,
}

impl NodeColumn {
    pub(crate) fn new(db: Arc<DB>) -> Self {
        Self {
            inner: db
        }
    }
}

impl DatabaseBackend for NodeColumn {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;
        self.inner
            .put_cf_opt(&cf, key, value, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;

        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        value.ok_or(StorageError::InvalidKey(key.to_vec()).into())
    }

    fn delete(&self, key: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;

        self.inner
            .delete_cf_opt(&cf, key, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;
        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        Ok(value.unwrap_or(default))
    }

    fn column_name() -> &'static str {
        "__node__"
    }
}


pub(crate) struct ValueColumn {
    inner: Arc<DB>,
}

impl ValueColumn {
    pub(crate) fn new(db: Arc<DB>) -> Self {
        Self {
            inner: db
        }
    }
}

impl DatabaseBackend for ValueColumn {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;
        self.inner
            .put_cf_opt(&cf, key, value, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;

        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        value.ok_or(StorageError::InvalidKey(key.to_vec()).into())
    }

    fn delete(&self, key: &[u8]) -> Result<()>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;

        self.inner
            .delete_cf_opt(&cf, key, &default_write_opts())
            .map_err(|e| e.into())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>
    {
        let cf = self
            .inner
            .cf_handle(Self::column_name())
            .ok_or(StorageError::ColumnFamilyMissing(Self::column_name()))?;
        let value = self.inner.get_cf_opt(&cf, &key, &default_read_opts())?;
        Ok(value.unwrap_or(default))
    }

    fn column_name() -> &'static str {
        "__value__"
    }
}


pub(crate) struct MapStore {
    inner: Arc<DashMap<Vec<u8>, Vec<u8>>>,
}

impl MapStore {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new())
        }
    }
}

impl DatabaseBackend for MapStore {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>
    {
        self.inner.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>
    {
        let value = self.inner.get(key).map(|r| r.value().clone());
        value.ok_or(StorageError::InvalidKey(key.to_vec()).into())
    }

    fn delete(&self, key: &[u8]) -> Result<()>
    {
        self.inner.remove(key);
        Ok(())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>
    {
        let value = self.inner.get(key).map(|r| r.value().clone());
        Ok(value.unwrap_or(default))
    }

    fn column_name() -> &'static str {
        ""
    }
}