use anyhow::Result;
use crate::transaction::Tx;
use crate::block::{BlockHeader, Block};
use crate::mempool::{MemPool, MemPoolStorageKV};
use std::sync::Arc;
use crate::utxo::{UTXO, UTXOStorageKV};
use crate::consensus::{validate_transaction, check_block_pow, execute_tx, validate_block};
use crate::errors::BlockChainError;
use crate::block_storage::{BlockStorage, BlockStorageKV};
use storage::{Storage, KVEntry};
use storage::codec::{Encoder, Decoder};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use storage::presistent_store::PersistentStore;

pub const BLOCK_DIFFICULTY: &str = "00000";
pub const GENESIS_BLOCK: &str = "0000004cf909c9a9c71ada661a3e4104e004db406f3dcdf25fd6e0aad664b15700000000000000000000000000000000000000000000000000000000000000000000000097f9bc610100a44f0000000000000000000000000000e972937e71e8b54595f3d9659050c3ee906337896a9eb96e74dfb0e702bb3d680100000000000000e536ea0295f24623b9ccbb5b96babbe586c8efd6f2745a19fcf7e3f195af63b8";

pub struct BlockChain {
    state :  BlockChainState
}

impl BlockChain {
    pub fn new(mempool : Arc<MemPool>, utxo : Arc<UTXO>, block_storage : Arc<BlockStorage>, state: Arc<BlockChainStateKV>) -> Self {
        let chain_state = BlockChainState {
            mempool,
            utxo,
            block_storage,
            state
        };

        Self {
            state: chain_state
        }
    }

}

const CURRENT_HEAD_KEY : &str = "current-head";

pub type BlockChainStateKV = dyn Storage<BlockChainState> + Send + Sync;

#[derive(Serialize, Deserialize)]
pub enum StateValue {
    CurrentHead(BlockHeader)
}

#[derive(Serialize, Deserialize)]
 pub struct  StateKey {
    key : String
}

impl Encoder for StateValue {}
impl Decoder for StateValue {}

impl Encoder for StateKey {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.key.into_bytes())
    }
}

impl Decoder for StateKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(StateKey {
            key: String::from_utf8(buf.to_vec())?
        })
    }
}

impl From<String> for StateKey {
    fn from(inner: String) -> Self {
        Self {
            key: inner
        }
    }
}

impl From<&'static str> for StateKey {
    fn from(s: &'static str) -> Self {
        Self {
            key: s.to_string()
        }
    }
}


pub struct BlockChainState {
    mempool: Arc<MemPool>,
    utxo: Arc<UTXO>,
    block_storage : Arc<BlockStorage>,
    state: Arc<BlockChainStateKV>
}

impl BlockChainState {
    pub fn new(mempool: Arc<MemPool>,
                utxo: Arc<UTXO>,
                block_storage : Arc<BlockStorage>,
                state: Arc<BlockChainStateKV> ) -> Self {

        Self {
            mempool,
            utxo,
            block_storage,
            state
        }
    }
}

impl KVEntry for BlockChainState {
    type Key = StateKey;
    type Value = StateValue;
}

impl BlockChainState {
    pub fn get_current_head(&self) -> Result<Option<BlockHeader>> {
        match self.state.get(&CURRENT_HEAD_KEY.into())? {
            None => {
                Ok(None)
            }
            Some(StateValue::CurrentHead(head)) => {
                Ok(Some(head))
            }
        }
    }

    pub fn resolve_current_head(&self, new_block : &Block) -> Result<()> {
        let current_head = self.get_current_head()?;
        match current_head {
            None => {
                self.state.put(CURRENT_HEAD_KEY.into(), StateValue::CurrentHead(new_block.header()))
            }
            Some(current_head) => {
                if current_head.block_hash() == new_block.prev_block_hash() {
                    self.state.put(CURRENT_HEAD_KEY.into(), StateValue::CurrentHead(new_block.header()))
                }
            }
        }
    }
    pub fn dispatch(&self, action : StateAction) -> Result<()> {
        match action {
            StateAction::AddNewBlock(block) => {
                validate_block(&block)?;
                for tx_hash in block.transactions().iter() {
                    let tx = self.mempool.get_tx(tx_hash)?.ok_or(BlockChainError::TransactionNotFound)?;
                    execute_tx(tx, self.utxo.as_ref())?;
                    self.mempool.remove(tx_hash);
                }

                self.block_storage.put_block(block)

            }
            StateAction::AddNewTransaction(tx) => {
                self.mempool.put(&tx)?;
            }
        }
        Ok(())
    }
}


pub enum StateAction {
    AddNewBlock(Block),
    AddNewTransaction(Tx)
}