use std::sync::Arc;

use anyhow::Result;

use primitive_types::{H160, Compact, H256, U256};
use types::{BlockHash, PubKey, TxHash, Hash, Genesis};
use types::account::AccountState;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::tx::Transaction;

pub trait StateDB: Send + Sync {
    fn nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
    fn balance(&self, address: &H160) -> u128;
    fn apply_transaction(&self, tx: &Transaction) -> Hash;
}

pub trait Saturating {
    fn saturating_add(self, rhs: Self) -> Self;

    fn saturating_sub(self, rhs: Self) -> Self;

    fn saturating_mul(self, rhs: Self) -> Self;
}


pub trait ChainHeadReader: Send + Sync {
    fn current_header(&self) -> Result<Option<IndexedBlockHeader>>;
    fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>>;
}

pub trait ChainReader: ChainHeadReader + Send + Sync {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>>;
}


pub trait Consensus: Send + Sync {
    const BLOCK_MAX_FUTURE: i64;
    const COINBASE_MATURITY: u32;
    // 2 hours
    const MIN_COINBASE_SIZE: usize;
    const MAX_COINBASE_SIZE: usize;

    const RETARGETING_FACTOR: u32;
    const TARGET_SPACING_SECONDS: u32;
    const DOUBLE_SPACING_SECONDS: u32;
    const TARGET_TIMESPAN_SECONDS: u32;

    // The upper and lower bounds for retargeting timespan
    const MIN_TIMESPAN: u32 = Self::TARGET_TIMESPAN_SECONDS / Self::RETARGETING_FACTOR;
    const MAX_TIMESPAN: u32 = Self::TARGET_TIMESPAN_SECONDS * Self::RETARGETING_FACTOR;

    // Target number of blocks, 2 weaks, 2016
    const RETARGETING_INTERVAL: u32 = Self::TARGET_TIMESPAN_SECONDS / Self::TARGET_SPACING_SECONDS;

    fn verify_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<()>;
    fn prepare_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<BlockHeader>;
    fn finalize(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<()>;
    fn finalize_and_assemble(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<Option<Block>>;
    fn work_required(&self, chain: Arc<dyn ChainHeadReader>, parent: &Hash, time: u32) -> Result<Compact>;
    fn is_genesis(&self, header: &BlockHeader) -> bool;
    fn miner_reward(&self, block_level: i32) -> u128;
}

pub fn is_valid_proof_of_work(max_work_bits: Compact, bits: Compact, hash: &H256) -> bool {
    let maximum = match max_work_bits.to_u256() {
        Ok(max) => max,
        _err => return false,
    };

    let target = match bits.to_u256() {
        Ok(target) => target,
        _err => return false,
    };

    let mut raw_hash = *hash.as_fixed_bytes();
    raw_hash.reverse();
    let value = U256::from(&raw_hash);
    target <= maximum && value <= target
}