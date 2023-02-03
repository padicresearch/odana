use std::sync::Arc;

use anyhow::Result;

use primitive_types::{Address, Compact, H160, H256};
use smt::SparseMerkleTree;
use types::account::AccountState;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::network::Network;
use types::receipt::Receipt;
use types::tx::{ApplicationCallTx, CreateApplicationTx, SignedTransaction};
use types::{Changelist, Hash};

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
    fn apply_txs(&self, vm: Arc<dyn WasmVMInstance>, txs: &[SignedTransaction]) -> Result<H256>;
    fn root(&self) -> Hash;
    fn commit(&self) -> Result<()>;
    fn snapshot(&self) -> Result<Arc<dyn StateDB>>;
    fn state_at(&self, root: H256) -> Result<Arc<dyn StateDB>>;
}

pub trait AppData: Send + Sync {
    fn get_app_data(&self, app_id: Address) -> Result<SparseMerkleTree>;
    fn set_app_data(&self, app_id: Address, app_data: SparseMerkleTree) -> Result<()>;
    fn get_app_root(&self, app_id: Address) -> H256;
    fn get_app_data_at_root(&self, app_id: Address, root: H256) -> Result<SparseMerkleTree>;
}

pub trait AccountStateReader: Send + Sync {
    fn nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
    fn balance(&self, address: &H160) -> u128;
}

pub trait WasmVMInstance: Send + Sync {
    fn execute_app_create(
        &self,
        state_db: Arc<dyn StateDB>,
        sender: Address,
        value: u64,
        call: &CreateApplicationTx,
    ) -> Result<Changelist>;
    fn execute_app_tx(
        &self,
        state_db: Arc<dyn StateDB>,
        sender: Address,
        value: u64,
        call: &ApplicationCallTx,
    ) -> Result<Changelist>;
    fn load_app(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address, //TODO; use codehash instead of app id
        binary: Vec<u8>,
    ) -> Result<()>;
    fn execute_app_query(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address,
        raw_query: &[u8],
    ) -> Result<Vec<u8>>;
}

pub trait StateIntermediate {}

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
        vm: Arc<dyn WasmVMInstance>,
        state: Arc<dyn StateDB>,
        txs: &[SignedTransaction],
    ) -> Result<()>;
    fn finalize_and_assemble(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        vm: Arc<dyn WasmVMInstance>,
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
