use commitlog::CommitLog;
use storage::{KVStore, KVEntry, PersistentStorage};
use anyhow::Result;
use crate::block::Block;
use common::BlockHash;
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
                    PersistentStorage::MemStore(storage) => {
                        storage.clone()
                    }
                    PersistentStorage::PersistentStore(storage) => {
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