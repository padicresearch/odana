use commitlog::CommitLog;
use storage::{KVStore, KVEntry, PersistentStorage};
use anyhow::Result;
use types::block::Block;
use types::BlockHash;
use std::sync::Arc;

pub type BlockStorageKV = dyn KVStore<BlockStorage> + Send + Sync;

pub struct BlockStorage {
    kv : Arc<BlockStorageKV>
}



impl BlockStorage {
    pub fn new(storage : Arc<PersistentStorage> ) -> Self {
        Self {
            kv: {
                match storage.as_ref() {
                    PersistentStorage::InMemory(storage) => {
                        storage.clone()
                    }
                    PersistentStorage::Sled(storage) => {
                        storage.clone()
                    }
                }
            }
        }
    }
    pub fn put_block(&self, block : Block) -> Result<()> {
        let block = block.clone();
        self.kv.put(*block.hash(), block)
    }

    pub fn get_block(&self, block_hash : &BlockHash) -> Result<Option<Block>> {
        self.kv.get(block_hash)
    }
}


impl KVEntry for BlockStorage {
    type Key = BlockHash;
    type Value = Block;

    fn column() -> &'static str {
        "block_storage"
    }
}