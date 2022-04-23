use anyhow::Result;
use rocksdb::{BlockBasedOptions, Cache, ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use std::clone;
use std::path::Path;
use std::sync::Arc;
use crate::persistent::{DiskStore, default_db_opts, MemoryStore};

const COLUMN_NODE: &'static str = "__node__";
const COLUMN_VALUE: &'static str = "__value__";

pub(crate) fn cfs() -> Vec<ColumnFamilyDescriptor> {
    vec![
        ColumnFamilyDescriptor::new(COLUMN_NODE, default_table_options()),
        ColumnFamilyDescriptor::new(COLUMN_VALUE, default_table_options()),
    ]
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





pub(crate) trait DatabaseBackend {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>;

    fn delete(&self, key: &[u8]) -> Result<()>;

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>;
}

pub(crate) struct Database {
    pub(crate) nodes: Arc<dyn DatabaseBackend + Send + Sync>,
    pub(crate) values: Arc<dyn DatabaseBackend + Send + Sync>,
}

impl Database {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), path, cfs())?);
        Ok(Self {
            nodes: Arc::new(DiskStore::new(COLUMN_NODE, db.clone())),
            values: Arc::new(DiskStore::new(COLUMN_VALUE, db.clone())),
        })
    }

    pub(crate) fn in_memory() -> Self {
        Self {
            nodes: Arc::new(MemoryStore::new()),
            values: Arc::new(MemoryStore::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn test(nodes: Arc<MemoryStore>, values: Arc<MemoryStore>) -> Self {
        Self {
            nodes,
            values,
        }
    }
}

