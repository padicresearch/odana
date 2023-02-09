use std::sync::Arc;

use anyhow::Result;

use primitive_types::H256;
use storage::{KVStore, PersistentStorage, Schema, StorageIterator};
use traits::{ChainHeadReader, ChainReader};
use types::block::{Block, BlockHeader, BlockPrimaryKey, IndexedBlockHeader};
use types::tx::{SignedTransaction, TransactionList};

pub struct BlockStorage {
    headers: Arc<BlockHeaderStorage>,
    transactions: Arc<BlockTransactionsStorage>,
    block_by_hash: Arc<BlockByHash>,
    block_by_level: Arc<BlockByLevel>,
}

impl BlockStorage {
    pub fn new(persistent: Arc<PersistentStorage>) -> Self {
        Self {
            headers: Arc::new(BlockHeaderStorage::new(persistent.database())),
            transactions: Arc::new(BlockTransactionsStorage::new(persistent.database())),
            block_by_hash: Arc::new(BlockByHash::new(persistent.database())),
            block_by_level: Arc::new(BlockByLevel::new(persistent.database())),
        }
    }

    pub fn put(&self, block: Block) -> Result<()> {
        let block_key = self.headers.put(*block.header())?;
        self.transactions
            .put(block_key, block.into_transactions())?;
        self.block_by_hash.put(block_key.1, block_key)?;
        self.block_by_level.put(block_key.0, block_key)?;
        Ok(())
    }

    pub fn delete(&self, hash: &H256, level: u32) -> Result<()> {
        let block_key = BlockPrimaryKey(level, *hash);
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
            self.headers
                .get_blocks(&primary_key)?
                .zip(self.transactions.get_block_transactions(&primary_key)?)
                .map(|((_, header), (_, transactions))| {
                    let header = header?;
                    let transaction_list = transactions?;
                    Ok(Block::new(header, transaction_list.into()))
                }),
        ))
    }
}

impl ChainHeadReader for BlockStorage {
    fn get_header(&self, hash: &H256, level: u32) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = BlockPrimaryKey(level, *hash);
        self.headers
            .get_blockheader(&primary_key)
            .map(|opt_block| opt_block.map(|b| b.into()))
    }

    fn get_header_by_hash(&self, hash: &H256) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = self.block_by_hash.get(hash)?;
        if let Some(primary_key) = primary_key {
            return self.get_header(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }

    fn get_header_by_level(&self, level: u32) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let primary_key = self.block_by_level.get(level)?;
        if let Some(primary_key) = primary_key {
            return self.get_header(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }
}

impl ChainReader for BlockStorage {
    fn get_block(&self, hash: &H256, level: u32) -> anyhow::Result<Option<Block>> {
        let primary_key = BlockPrimaryKey(level, *hash);
        let (Some(header),Some(transactions)) = (self.headers.get_blockheader(&primary_key)?, self.transactions.get_transactions(&primary_key)?) else {
            return Ok(None)
        };
        Ok(Some(Block::new(header, transactions.into())))
    }

    fn get_block_by_hash(&self, hash: &H256) -> anyhow::Result<Option<Block>> {
        let primary_key = self.block_by_hash.get(hash)?;
        if let Some(primary_key) = primary_key {
            return self.get_block(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }

    fn get_block_by_level(&self, level: u32) -> Result<Option<Block>> {
        let primary_key = self.block_by_level.get(level)?;
        if let Some(primary_key) = primary_key {
            return self.get_block(&primary_key.1, primary_key.0);
        }
        Ok(None)
    }
}

/// Primary block storage
pub type BlockHeaderStorageKV = dyn KVStore<BlockHeaderStorage> + Send + Sync;

//TODO: store transaction hashes as part of header
pub struct BlockHeaderStorage {
    kv: Arc<BlockHeaderStorageKV>,
}

impl Schema for BlockHeaderStorage {
    type Key = BlockPrimaryKey;
    type Value = BlockHeader;

    fn column() -> &'static str {
        "block_storage"
    }
}

impl BlockHeaderStorage {
    pub fn new(kv: Arc<BlockHeaderStorageKV>) -> Self {
        Self { kv }
    }
    pub fn put(&self, blockheader: BlockHeader) -> Result<BlockPrimaryKey> {
        let hash = blockheader.hash();
        let level = blockheader.level();
        let block_key = BlockPrimaryKey(level, hash);
        if self.kv.contains(&block_key)? {
            return Ok(block_key);
        }
        self.kv.put(block_key, blockheader)?;
        Ok(block_key)
    }
    pub fn get_blockheader(&self, block_key: &BlockPrimaryKey) -> Result<Option<BlockHeader>> {
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
    ) -> Result<StorageIterator<BlockHeaderStorage>> {
        self.kv.prefix_iter(start_at)
    }
}

pub type BlockTransactionsStorageKV = dyn KVStore<BlockTransactionsStorage> + Send + Sync;

//TODO: store transaction hash
pub struct BlockTransactionsStorage {
    kv: Arc<BlockTransactionsStorageKV>,
}

impl Schema for BlockTransactionsStorage {
    type Key = BlockPrimaryKey;
    type Value = TransactionList;

    fn column() -> &'static str {
        "block_transactions_storage"
    }
}

impl BlockTransactionsStorage {
    pub fn new(kv: Arc<BlockTransactionsStorageKV>) -> Self {
        Self { kv }
    }
    pub fn put(&self, block_key: BlockPrimaryKey, txs: Vec<SignedTransaction>) -> Result<()> {
        self.kv.put(block_key, TransactionList::from(txs))?;
        Ok(())
    }
    pub fn get_transactions(&self, block_key: &BlockPrimaryKey) -> Result<Option<TransactionList>> {
        self.kv.get(block_key)
    }

    pub fn delete_block(&self, block_key: &BlockPrimaryKey) -> Result<()> {
        self.kv.delete(block_key)
    }

    pub fn has_block(&self, block_key: &BlockPrimaryKey) -> Result<bool> {
        self.kv.contains(block_key)
    }

    pub fn get_block_transactions(
        &self,
        start_at: &BlockPrimaryKey,
    ) -> Result<StorageIterator<BlockTransactionsStorage>> {
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
        self.kv.delete(&key)
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
