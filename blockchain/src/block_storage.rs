use commitlog::CommitLog;
use storage::{Storage, KVEntry};
use anyhow::Result;
use crate::block::Block;
use common::BlockHash;
use std::sync::Arc;

pub type BlockStorageKV = dyn Storage<BlockStorage> + Send + Sync;

pub struct BlockStorage {
    kv : Arc<BlockStorageKV>
}

impl BlockStorage {
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
}