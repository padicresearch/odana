use crate::{KVStore, KVEntry, StorageIterator};

struct RocksDB {}

impl<S: KVEntry> KVStore<S> for rocksdb::DB {
    fn get(&self, key: &crate::Key) -> anyhow::Result<Option<crate::Value>> {
        self.get_cf()
    }

    fn put(&self, key: crate::Key, value: crate::Value) -> anyhow::Result<()> {
        todo!()
    }

    fn delete(&self, key: &crate::Key) -> anyhow::Result<()> {
        todo!()
    }

    fn contains(&self, key: &crate::Key) -> anyhow::Result<bool> {
        todo!()
    }

    fn iter(&self) -> anyhow::Result<StorageIterator<S>> {
        todo!()
    }
}