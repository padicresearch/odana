use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use rocksdb::{BlockBasedOptions, ColumnFamilyDescriptor, Options};

use codec::Codec;

use crate::memstore::MemStore;
use crate::sleddb::SledDB;

pub mod error;
pub mod memstore;
mod rocks;
pub mod sleddb;

pub fn default_table_options() -> Options {
    // default db options
    let mut db_opts = Options::default();

    // https://github.com/facebook/rocksdb/wiki/Setup-Options-and-Basic-Tuning#other-general-options
    db_opts.set_level_compaction_dynamic_level_bytes(false);
    db_opts.set_write_buffer_size(32 * 1024 * 1024);

    // block table options
    let mut table_options = BlockBasedOptions::default();
    // table_options.set_block_cache(cache);
    // table_options.set_block_size(16 * 1024);
    // table_options.set_cache_index_and_filter_blocks(true);
    // table_options.set_pin_l0_filter_and_index_blocks_in_cache(true);

    // set format_version 4 https://rocksdb.org/blog/2019/03/08/format-version-4.html
    table_options.set_format_version(4);
    // table_options.set_index_block_restart_interval(16);

    db_opts.set_block_based_table_factory(&table_options);

    db_opts
}

pub trait Schema {
    type Key: Codec + Clone;
    type Value: Codec;
    fn column() -> &'static str;
    fn descriptor() -> ColumnFamilyDescriptor {
        ColumnFamilyDescriptor::new(Self::column(), default_table_options())
    }
}

pub enum PersistentStorageBackend {
    InMemory(Arc<MemStore>),
    Sled(Arc<SledDB>),
    RocksDB(Arc<rocksdb::DB>),
}

pub struct PersistentStorage {
    backend: PersistentStorageBackend,
}

impl PersistentStorage {
    pub fn new(backend: PersistentStorageBackend) -> Self {
        Self { backend }
    }

    pub fn database<S>(&self) -> Arc<dyn KVStore<S> + Send + Sync>
    where
        S: Schema,
    {
        match &self.backend {
            PersistentStorageBackend::InMemory(database) => database.clone(),
            PersistentStorageBackend::Sled(database) => database.clone(),
            PersistentStorageBackend::RocksDB(database) => database.clone(),
        }
    }
}

pub trait KVStore<Entry>
where
    Entry: Schema,
{
    fn get(&self, key: &Entry::Key) -> Result<Option<Entry::Value>>;
    fn put(&self, key: Entry::Key, value: Entry::Value) -> Result<()>;
    fn delete(&self, key: &Entry::Key) -> Result<()>;
    fn contains(&self, key: &Entry::Key) -> Result<bool>;
    fn iter(&self) -> Result<StorageIterator<Entry>>;
}

pub type StorageIterator<'a, Entry: Schema> =
    Box<dyn 'a + Send + Iterator<Item = (Result<Entry::Key>, Result<Entry::Value>)>>;

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use anyhow::Result;
    use rocksdb::ColumnFamilyDescriptor;
    use tempdir::TempDir;

    use crate::{KVStore, PersistentStorage, PersistentStorageBackend, Schema};
    use crate::sleddb::SledDB;

    pub type BlockStorageKV = dyn KVStore<BlockStorage> + Send + Sync;

    pub struct BlockStorage {
        kv: Arc<BlockStorageKV>,
    }

    impl BlockStorage {
        pub fn new(kv: Arc<BlockStorageKV>) -> Self {
            Self { kv }
        }
        pub fn put_block(&self, key: String, block: String) -> Result<()> {
            self.kv.put(key, block)
        }
        pub fn get_block(&self, key: &String) -> Result<Option<String>> {
            self.kv.get(key)
        }
    }

    impl Schema for BlockStorage {
        type Key = String;
        type Value = String;

        fn column() -> &'static str {
            "block_storage"
        }

        fn descriptor() -> ColumnFamilyDescriptor {
            todo!()
        }
    }

    #[test]
    fn test_backends() {
        let temp = TempDir::new("_test_backends_").unwrap();
        let sled = Arc::new(SledDB::new(temp.path()).unwrap());
        let persistent = Arc::new(PersistentStorage::new(PersistentStorageBackend::Sled(sled)));
        let block_storage = BlockStorage::new(persistent.database());
        block_storage
            .put_block("h".to_string(), "bb".to_string())
            .unwrap();
        println!("{:?}", block_storage.get_block(&"h".to_string()).unwrap())
    }
}
