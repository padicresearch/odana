use std::sync::Arc;

use anyhow::Result;

use primitive_types::{Compact, H160, H256};
use types::account::{AccountState, Address};
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::tx::{ApplicationCallTx, SignedTransaction};
use types::Hash;
use types::receipt::Receipt;

pub trait Blockchain: ChainReader {
    fn get_current_state(&self) -> Result<Arc<dyn StateDB>>;
    fn current_header(&self) -> Result<Option<IndexedBlockHeader>>;
    fn get_state_at(&self, root: &H256) -> Result<Arc<dyn StateDB>>;
}

pub trait StateDB: Send + Sync {
    fn nonce(&self, address: &Address) -> u64;
    fn account_state(&self, address: &Address) -> AccountState;
    fn balance(&self, address: &Address) -> u64;
    fn credit_balance(&self, address: &Address, amount: u64) -> Result<H256>;
    fn debit_balance(&self, address: &Address, amount: u64) -> Result<H256>;
    fn reset(&self, root: H256) -> Result<()>;
    fn apply_txs(&self, txs: &[SignedTransaction]) -> Result<H256>;
    fn root(&self) -> Hash;
    fn commit(&self) -> Result<()>;
    fn snapshot(&self) -> Result<Arc<dyn StateDB>>;
    fn state_at(&self, root: H256) -> Result<Arc<dyn StateDB>>;
}

pub trait ContextDB: Send + Sync {
    fn app_state(&self, app_id: u32) -> Result<Arc<dyn ContextTrieDB>>;
    fn app_state_root(&self, app_id: u32) -> H256;
    fn state_at(&self, app_id: u32, root: H256) -> Result<Arc<dyn ContextTrieDB>>;
}

pub trait ContextTrieDB: Send + Sync {
    fn put(&self, key: &[u8], value: &[u8]);
    fn delete(&self, key: &[u8]);
    fn get(&self, key: &[u8]) -> H256;
    fn root(&self) -> H256;
}

pub trait AccountStateReader: Send + Sync {
    fn nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
    fn balance(&self, address: &H160) -> u128;
}

pub trait WasmVMInstance: Send + Sync {
    fn execute_app_tx(&self, call: &ApplicationCallTx) -> Result<Receipt>;
    fn execute_app_query(&self, app_id: u32, raw_query: &[u8]) -> Result<Receipt>;
    fn verify_call(&self, call: &ApplicationCallTx) -> bool;
    fn estimate_call_cost(&self, call: &ApplicationCallTx) -> u64;
}

pub trait StateIntermediate {}

pub trait Saturating {
    fn saturating_add(self, rhs: Self) -> Self;

    fn saturating_sub(self, rhs: Self) -> Self;

    fn saturating_mul(self, rhs: Self) -> Self;
}

pub trait ChainHeadReader: Send + Sync {
    fn get_header(&self, hash: &H256, level: u32) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_hash(&self, hash: &H256) -> Result<Option<IndexedBlockHeader>>;
    fn get_header_by_level(&self, level: u32) -> Result<Option<IndexedBlockHeader>>;
}

pub trait ChainReader: Send + Sync {
    fn get_block(&self, hash: &H256, level: u32) -> Result<Option<Block>>;
    fn get_block_by_hash(&self, hash: &H256) -> Result<Option<Block>>;
    fn get_block_by_level(&self, level: u32) -> Result<Option<Block>>;
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
        txs: &[SignedTransaction],
    ) -> Result<()>;
    fn finalize_and_assemble(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        state: Arc<dyn StateDB>,
        txs: &[SignedTransaction],
    ) -> Result<Option<Block>>;
    fn work_required(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        parent: &H256,
        time: u32,
    ) -> Result<Compact>;
    fn is_genesis(&self, header: &BlockHeader) -> bool;
    fn miner_reward(&self, block_level: u32) -> u64;
    fn get_genesis_header(&self) -> BlockHeader;
}

pub trait Handler<T> {
    fn handle(&mut self, msg: T);
}
