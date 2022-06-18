use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use primitive_types::{Compact, H160, H256, U256};
use types::account::AccountState;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::tx::Transaction;
use types::{Genesis, Hash};

pub trait Blockchain: ChainReader {
    fn get_current_state(&self) -> Result<Arc<dyn StateDB>>;
    fn current_header(&self) -> Result<Option<IndexedBlockHeader>>;
    fn get_state_at(&self, root: &types::Hash) -> Result<Arc<dyn StateDB>>;
}

pub trait StateDB: Send + Sync {
    fn nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
    fn balance(&self, address: &H160) -> u128;
    fn credit_balance(&self, address: &H160, amount: u128) -> Result<Hash>;
    fn debit_balance(&self, address: &H160, amount: u128) -> Result<Hash>;
    fn reset(&self, root: H256) -> Result<()>;
    fn apply_txs(&self, txs: Vec<Transaction>) -> Result<Hash>;
    fn root(&self) -> Hash;
    fn commit(&self) -> Result<()>;
    fn snapshot(&self) -> Result<Arc<dyn StateDB>>;
}

pub trait StateIntermediate {}

pub trait Saturating {
    fn saturating_add(self, rhs: Self) -> Self;

    fn saturating_sub(self, rhs: Self) -> Self;

    fn saturating_mul(self, rhs: Self) -> Self;
}

pub trait ChainHeadReader: Send + Sync {
    fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>>;
}

pub trait ChainReader: Send + Sync {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>>;
    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>>;
}

pub trait Consensus: Send + Sync {
    fn verify_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<()>;
    fn prepare_header(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
    ) -> Result<()>;
    fn finalize(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        state: Arc<dyn StateDB>,
        txs: Vec<Transaction>,
    ) -> Result<()>;
    fn finalize_and_assemble(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        state: Arc<dyn StateDB>,
        txs: Vec<Transaction>,
    ) -> Result<Option<Block>>;
    fn work_required(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        parent: &Hash,
        time: u32,
    ) -> Result<Compact>;
    fn is_genesis(&self, header: &BlockHeader) -> bool;
    fn miner_reward(&self, block_level: i32) -> u128;
    fn get_genesis_header(&self) -> BlockHeader;
}
