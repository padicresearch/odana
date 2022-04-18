use anyhow::Result;
use rocksdb::{BlockBasedOptions, Cache, ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use std::clone;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use crate::persistent::{cfs, default_db_opts, MapStore, NodeColumn, ValueColumn};

const COLUMN_NODE: &'static str = "node_map";
const COLUMN_VALUE: &'static str = "value_map";

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("RWPoison")]
    RWPoison,
    #[error("ColumnFamilyMissing {0}")]
    ColumnFamilyMissing(&'static str),

    #[error("Invalid Key {0:#?}")]
    InvalidKey(Vec<u8>),
}


pub(crate) trait DatabaseBackend {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    fn get(&self, key: &[u8]) -> Result<Vec<u8>>;

    fn delete(&self, key: &[u8]) -> Result<()>;

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> Result<Vec<u8>>;

    fn column_name() -> &'static str where Self: Sized;
}

pub(crate) struct Database {
    pub(crate) nodes: Arc<dyn DatabaseBackend + Send + Sync>,
    pub(crate) value: Arc<dyn DatabaseBackend + Send + Sync>,
}

impl Database {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), path, cfs())?);
        Ok(Self {
            nodes: Arc::new(NodeColumn::new(db.clone())),
            value: Arc::new(ValueColumn::new(db.clone())),
        })
    }

    pub(crate) fn in_memory() -> Self {
        Self {
            nodes: Arc::new(MapStore::new()),
            value: Arc::new(MapStore::new()),
        }
    }
}

