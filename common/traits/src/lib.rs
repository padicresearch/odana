use std::sync::Arc;

use anyhow::Result;

use primitive_types::{H160, Compact};
use types::{BlockHash, PubKey, TxHash, Hash, Genesis};
use types::account::AccountState;
use types::block::{Block, BlockHeader};
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
    fn current_header(&self) -> Result<Option<BlockHeader>>;
    fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<BlockHeader>>;
    fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<BlockHeader>>;
    fn get_header_by_level(&self, level: i32) -> Result<Option<BlockHeader>>;
}

pub trait ChainReader: ChainHeadReader + Send + Sync {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>>;
}


pub trait Consensus: Send + Sync {
    fn verify_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<()>;
    fn prepare_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<BlockHeader>;
    fn finalize(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<()>;
    fn finalize_and_assemble(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<Option<Block>>;
    fn calc_difficulty(&self, level: i32, parent: &BlockHeader) -> Compact;
    fn is_genesis(&self, header: &BlockHeader) -> bool;
}