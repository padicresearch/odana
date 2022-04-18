use anyhow::Result;
use rocksdb::{BlockBasedOptions, Cache, ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use std::clone;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use crate::persistent::{cfs, default_db_opts, NodeColumn, ValueColumn};

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
    const COLUMN_NAME: &'static str;
    fn put<K, V>(&self, key: K, value: V) -> Result<()>
        where
            K: AsRef<[u8]>,
            V: AsRef<[u8]>;

    fn get<K>(&self, key: K) -> Result<Vec<u8>>
        where
            K: AsRef<[u8]>;

    fn delete<K>(&self, key: K) -> Result<()>
        where
            K: AsRef<[u8]>;

    fn get_or_default<K>(&self, key: K, default: Vec<u8>) -> Result<Vec<u8>>
        where
            K: AsRef<[u8]>;
}

pub(crate) struct Database {
    pub(crate) nodes: Arc<NodeColumn>,
    pub(crate) value: Arc<ValueColumn>,
}

impl Database {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), path, cfs())?);
        Ok(Self {
            nodes: Arc::new(NodeColumn::new(db.clone())),
            value: Arc::new(ValueColumn::new(db.clone())),
        })
    }
}

