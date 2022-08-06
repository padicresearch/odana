use crate::error::Error;
use crate::persistent::{default_db_opts, MemoryStore, RocksDB};
use crate::SparseMerkleTree;
use anyhow::{bail, Result};
use codec::{Decoder, Encoder};
use dashmap::DashMap;
use hex::ToHex;
use primitive_types::H256;
use rocksdb::{BlockBasedOptions, ColumnFamilyDescriptor, Options};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const COLUMN_TREES: &str = "t";
const COLUMN_ROOT: &str = "r";

pub(crate) fn cfs() -> Vec<ColumnFamilyDescriptor> {
    vec![
        ColumnFamilyDescriptor::new(COLUMN_TREES, default_table_options()),
        ColumnFamilyDescriptor::new(COLUMN_ROOT, default_table_options()),
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
    fn put(&self, column_name: &'static str, key: &[u8], value: &[u8]) -> Result<()>;

    fn get(&self, column_name: &'static str, key: &[u8]) -> Result<Vec<u8>>;

    fn delete(&self, column_name: &'static str, key: &[u8]) -> Result<()>;

    fn checkpoint(&self, path: PathBuf) -> Result<Arc<dyn DatabaseBackend + Send + Sync>>;

    fn get_or_default(
        &self,
        column_name: &'static str,
        key: &[u8],
        default: Vec<u8>,
    ) -> Result<Vec<u8>>;
}

pub(crate) struct Database {
    pub(crate) inner: Arc<dyn DatabaseBackend + Send + Sync>,
}

impl Database {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(
            &default_db_opts(),
            path.as_ref(),
            cfs(),
        )?);

        Ok(Self {
            inner: Arc::new(RocksDB::new(db)),
        })
    }

    pub(crate) fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_for_read_only(
            &default_db_opts(),
            path,
            vec![COLUMN_ROOT, COLUMN_TREES],
            false,
        )?);
        Ok(Self {
            inner: Arc::new(RocksDB::new(db)),
        })
    }

    pub(crate) fn in_memory() -> Self {
        Self {
            inner: Arc::new(MemoryStore::new()),
        }
    }

    pub(crate) fn put(&self, key: H256, value: SparseMerkleTree) -> Result<()> {
        self.inner
            .put(COLUMN_TREES, &key.encode()?, &value.encode()?)
    }

    pub(crate) fn set_root(&self, new_root: H256) -> Result<()> {
        self.inner.put(COLUMN_ROOT, b"root", &new_root.encode()?)
    }

    pub(crate) fn load_root(&self) -> Result<SparseMerkleTree> {
        let root = self.inner.get(COLUMN_ROOT, b"root")?;
        let root = H256::decode(&root)?;
        self.get(&root)
    }

    pub(crate) fn get(&self, key: &H256) -> Result<SparseMerkleTree> {
        SparseMerkleTree::decode(&self.inner.get(COLUMN_TREES, &key.encode()?)?)
    }

    pub(crate) fn delete(&self, key: &H256) -> Result<()> {
        self.inner.delete(COLUMN_TREES, &key.encode()?)
    }

    pub(crate) fn checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<Database> {
        Ok(Database {
            inner: self.inner.checkpoint(PathBuf::new().join(path.as_ref()))?,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ArchivedStorage {
    inner: DashMap<Vec<u8>, Vec<u8>>,
}

impl ArchivedStorage {
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.inner.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let value = self.inner.get(key).map(|r| r.value().clone());
        value.ok_or(Error::InvalidKey(key.encode_hex::<String>()).into())
    }

    pub fn delete(&self, key: &[u8]) -> Result<()> {
        if !self.inner.contains_key(key) {
            bail!(Error::InvalidKey(key.encode_hex::<String>()))
        }
        self.inner.remove(key);
        Ok(())
    }

    pub fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>> {
        let value = self.inner.get(key).map(|r| r.value().clone());
        Ok(value.unwrap_or(default))
    }
}
