use crate::persistent::{default_db_opts, RocksDB};
use crate::store::{cfs, DatabaseBackend, TrieCacheDatabase};
use anyhow::Result;
use codec::Codec;
use dashmap::DashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

pub struct KvDB<K, V> {
    inner: Arc<dyn DatabaseBackend + Send + Sync>,
    read_only: bool,
    staging: DashMap<Vec<u8>, Vec<u8>>,
    _data: PhantomData<(K, V)>,
}

impl<K, V> KvDB<K, V>
where
    K: Codec,
    V: Codec,
{
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open(&default_db_opts(), path.as_ref())?);
        Ok(Self {
            inner: Arc::new(RocksDB::new(db)),
            read_only: false,
            staging: Default::default(),
            _data: Default::default(),
        })
    }

    pub fn open_read_only_at_root<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_for_read_only(
            &default_db_opts(),
            path.as_ref(),
            false,
        )?);
        Ok(Self {
            inner: Arc::new(RocksDB::new(db)),
            read_only: true,
            staging: Default::default(),
            _data: Default::default(),
        })
    }

    pub(crate) fn put(&self, key: K, value: V) -> Result<()> {
        let key = key.encode()?;
        let value = value.encode()?;
        if self.read_only {
            println!("Stored KV in memory");
            self.staging.insert(key, value);
            return Ok(());
        }
        println!("Stored KV in persistence");
        self.inner.put(&key, &value)
    }

    pub(crate) fn get(&self, key: &K) -> Result<V> {
        let key = key.encode()?;
        if let Some(entry) = self.staging.get(&key) {
            return V::decode(entry.value());
        }
        let raw = self.inner.get(&key)?;
        V::decode(&raw)
    }

    fn delete(&self, key: &K) -> Result<()> {
        let key = key.encode()?;
        if self.read_only {
            self.staging.remove(&key);
            return Ok(());
        }
        self.inner.delete(&key)
    }
}
