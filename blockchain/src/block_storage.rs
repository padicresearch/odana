use std::sync::Arc;

use anyhow::Result;

use primitive_types::H256;
use storage::{KVStore, PersistentStorage, Schema, StorageIterator};
use traits::{ChainHeadReader, ChainReader};
use types::block::{Block, BlockPrimaryKey, IndexedBlockHeader};
use types::Hash;

pub struct BlockStorage {
    primary: Arc<BlockPrimaryStorage>,
    block_by_hash: Arc<BlockByHash>,
    block_by_level: Arc<BlockByLevel>,
}

impl BlockStorage {
    pub fn new(persistent: Arc<PersistentStorage>) -> Self {
        Self {
            primary: Arc::new(BlockPrimaryStorage::new(persistent.database())),
            block_by_hash: Arc::new(BlockByHash::new(persistent.database())),
            block_by_level: Arc::new(BlockByLevel::new(persistent.database())),
        }
    }

    pub fn put(&self, block: Block) -> Result<()> {
        let block_key = self.primary.put(block)?;
        self.block_by_hash.put(block_key.1, block_key)?;
        self.block_by_level.put(block_key.0, block_key)?;
        Ok(())
    }

    pub fn delete(&self, hash: &H256, level: u32) -> Result<()> {
        let block_key = BlockPrimaryKey( level, *hash);
        self.block_by_level.delete(block_key.0)?;
        Ok(())
    }

    pub fn get_blocks<'a>(
        &'a self,
        hash: &'a H256,
        level: u32,
    ) -> Result<Box<dyn 'a + Send + Iterator<Item = Result<Block>>>> {
        let primary_key = BlockPrimaryKey(level, *hash);
        Ok(Box::new(
            self.primary.get_blocks(&primary_key)?.map(|(_k, v)| v),
        ))
    }
}

impl ChainHeadReader for BlockStorage {
    fn get_header(&self, hash: &H256, level: u32) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = BlockPrimaryKey(level, *hash);
        self.primary
            .get_block(&primary_key)
            .map(|opt_block| opt_block.map(|b| (*b.header()).into()))
    }

    fn get_header_by_hash(&self, hash: &H256) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = self.block_by_hash.get(hash)?;
        if let Some(primary_key) = primary_key {
            return self.get_header(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }

    fn get_header_by_level(&self, level: i32) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = self.block_by_level.get(level)?;
        if let Some(primary_key) = primary_key {
            return self.get_header(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }
}

impl ChainReader for BlockStorage {
    fn get_block(&self, hash: &H256, level: u32) -> anyhow::Result<Option<Block>> {
        let primary_key = BlockPrimaryKey( level, *hash);
        self.primary.get_block(&primary_key)
    }

    fn get_block_by_hash(&self, hash: &H256) -> anyhow::Result<Option<Block>> {
        let primary_key = self.block_by_hash.get(hash)?;
        if let Some(primary_key) = primary_key {
            return self.get_block(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }

    fn get_block_by_level(&self, level: i32) -> Result<Option<Block>> {
        let primary_key = self.block_by_level.get(level)?;
        if let Some(primary_key) = primary_key {
            return self.get_block(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }
}

/// Primary block storage
pub type BlockPrimaryStorageKV = dyn KVStore<BlockPrimaryStorage> + Send + Sync;

pub struct BlockPrimaryStorage {
    kv: Arc<BlockPrimaryStorageKV>,
}

impl Schema for BlockPrimaryStorage {
    type Key = BlockPrimaryKey;
    type Value = Block;

    fn column() -> &'static str {
        "block_storage"
    }
}

impl BlockPrimaryStorage {
    pub fn new(kv: Arc<BlockPrimaryStorageKV>) -> Self {
        Self { kv }
    }
    pub fn put(&self, block: Block) -> Result<BlockPrimaryKey> {
        let hash = block.hash();
        let level = block.level();
        let block_key = BlockPrimaryKey(level, hash);
        if self.kv.contains(&block_key)? {
            return Ok(block_key);
        }
        self.kv.put(block_key, block)?;
        Ok(block_key)
    }
    pub fn get_block(&self, block_key: &BlockPrimaryKey) -> Result<Option<Block>> {
        self.kv.get(block_key)
    }

    pub fn delete_block(&self, block_key: &BlockPrimaryKey) -> Result<()> {
        self.kv.delete(block_key)
    }

    pub fn has_block(&self, block_key: &BlockPrimaryKey) -> Result<bool> {
        self.kv.contains(block_key)
    }

    pub fn get_blocks(
        &self,
        start_at: &BlockPrimaryKey,
    ) -> Result<StorageIterator<BlockPrimaryStorage>> {
        self.kv.prefix_iter(start_at)
    }
}

/// Block by level index
pub type BlockByLevelStorageKV = dyn KVStore<BlockByLevel> + Send + Sync;

pub struct BlockByLevel {
    kv: Arc<BlockByLevelStorageKV>,
}

impl Schema for BlockByLevel {
    type Key = u32;
    type Value = BlockPrimaryKey;

    fn column() -> &'static str {
        "block_level_storage"
    }
}

impl BlockByLevel {
    pub fn new(kv: Arc<BlockByLevelStorageKV>) -> Self {
        Self { kv }
    }
    pub fn put(&self, level: u32, primary_key: BlockPrimaryKey) -> Result<()> {
        if !self.kv.contains(&level)? {
            return self.kv.put(level, primary_key);
        }
        Ok(())
    }
    pub fn get(&self, level: u32) -> Result<Option<BlockPrimaryKey>> {
        self.kv.get(&level)
    }

    pub fn delete(&self, key: u32) -> Result<()> {
        self.kv.delete(&(key as u32))
    }
}

/// Block by hash index
pub type BlockByHashStorageKV = dyn KVStore<BlockByHash> + Send + Sync;

pub struct BlockByHash {
    kv: Arc<BlockByHashStorageKV>,
}

impl Schema for BlockByHash {
    type Key = H256;
    type Value = BlockPrimaryKey;

    fn column() -> &'static str {
        "block_hash_storage"
    }
}

impl BlockByHash {
    pub fn new(kv: Arc<BlockByHashStorageKV>) -> Self {
        Self { kv }
    }
    pub fn put(&self, hash: H256, primary_key: BlockPrimaryKey) -> Result<()> {
        self.kv.put(hash, primary_key)
    }
    pub fn delete(&self, hash: &H256) -> Result<()> {
        self.kv.delete(hash)
    }
    pub fn get(&self, hash: &H256) -> Result<Option<BlockPrimaryKey>> {
        self.kv.get(hash)
    }
}
