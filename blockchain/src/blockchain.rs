use crate::block_storage::{BlockStorage, BlockStorageKV};
use crate::consensus::{check_block_pow, execute_tx, validate_block, validate_transaction};
use crate::errors::BlockChainError;
use crate::mempool::{MemPool, MemPoolStorageKV, MempoolSnapsot};
use crate::miner::Miner;
use crate::transaction::Tx;
use crate::utxo::{UTXOStorageKV, UTXO};
use anyhow::{Error, Result};
use codec::{Decoder, Encoder};
use serde::{Deserialize, Serialize};
use std::f32::consts::E;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use storage::sleddb::SledDB;
use storage::{KVEntry, KVStore, PersistentStorage};
use tokio::sync::mpsc::UnboundedSender;
use types::block::{Block, BlockHeader};

pub const PROTOCOL_VERSION: u16 = 10001;
pub const SOFTWARE_VERSION: u16 = 10001;
pub const BLOCK_DIFFICULTY: &str = "000000";
pub const GENESIS_BLOCK: &str = "0000004cf909c9a9c71ada661a3e4104e004db406f3dcdf25fd6e0aad664b15700000000000000000000000000000000000000000000000000000000000000000000000097f9bc610100a44f0000000000000000000000000000e972937e71e8b54595f3d9659050c3ee906337896a9eb96e74dfb0e702bb3d680100000000000000e536ea0295f24623b9ccbb5b96babbe586c8efd6f2745a19fcf7e3f195af63b8";

pub fn genesis_block() -> Block {
    let decoded_bytes = hex::decode(GENESIS_BLOCK).expect("Error creating genesis block");
    let genesis_block: Block =
        bincode::deserialize(&decoded_bytes).expect("Error creating genesis block");
    genesis_block
}

pub struct BlockChain {
    state: Arc<BlockChainState>,
    miner: Arc<Miner>,
    local_mpsc_sender: UnboundedSender<LocalMessage>,
}
#[derive(Clone, Debug)]
pub enum LocalMessage {
    MindedBlock(Block),
    BroadcastTx(Tx),
    StateChanged {
        current_head: BlockHeader,
        mempool: MempoolSnapsot,
    },
}

impl BlockChain {
    pub fn new(
        storage: Arc<PersistentStorage>,
        local_mpsc_sender: UnboundedSender<LocalMessage>,
    ) -> Result<Self> {
        let utxo = Arc::new(UTXO::new(storage.clone()));
        let mempool = Arc::new(MemPool::new(utxo.clone(), storage.clone(), None)?);
        let block_storage = Arc::new(BlockStorage::new(storage.clone()));
        let miner = Arc::new(Miner::new(
            mempool.clone(),
            utxo.clone(),
            local_mpsc_sender.clone(),
        ));
        let chain_state = BlockChainState {
            mempool,
            utxo,
            block_storage,
            state: match storage.as_ref() {
                PersistentStorage::InMemory(storage) => storage.clone(),
                PersistentStorage::Sled(storage) => storage.clone(),
            },
        };
        chain_state.resolve_current_head(&genesis_block())?;

        Ok(Self {
            state: Arc::new(chain_state),
            miner,
            local_mpsc_sender,
        })
    }

    pub fn dispatch(&self, action: StateAction) -> Result<()> {
        match self.state.dispatch(action)? {
            StateEffect::CurrentHeadChanged => {
                if let (Ok(Some(current_head)), Ok(mempool)) =
                    (self.state.get_current_head(), self.state.get_mempool())
                {
                    self.local_mpsc_sender.send(LocalMessage::StateChanged {
                        current_head,
                        mempool,
                    })?;
                }
            }
            StateEffect::TxAdded => {}
            StateEffect::None => {}
        }
        Ok(())
    }

    pub fn state(&self) -> Arc<BlockChainState> {
        self.state.clone()
    }

    pub fn miner(&self) -> Arc<Miner> {
        self.miner.clone()
    }
}

pub fn start_mining(
    miner: Arc<Miner>,
    state: Arc<BlockChainState>,
    sender: UnboundedSender<LocalMessage>,
) {
    tokio::task::spawn(async move {
        loop {
            let state = state.clone();
            match miner.mine(
                &state
                    .get_current_head()
                    .expect("Blockchain state failed")
                    .ok_or(BlockChainError::UnknownError)
                    .expect("Blockchain state failed"),
            ) {
                Ok(new_block) => {
                    sender.send(LocalMessage::MindedBlock(new_block));
                }
                Err(error) => {
                    println!("Miner Error: {}", error);
                }
            }
        }
    });
}

const CURRENT_HEAD_KEY: &str = "current-head";

pub type BlockChainStateKV = dyn KVStore<BlockChainState> + Send + Sync;

#[derive(Serialize, Deserialize)]
pub enum StateValue {
    CurrentHead(BlockHeader),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StateKey {
    key: String,
}

impl Encoder for StateValue {}
impl Decoder for StateValue {}

impl Encoder for StateKey {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.key.as_bytes().to_vec())
    }
}

impl Decoder for StateKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(StateKey {
            key: String::from_utf8(buf.to_vec())?,
        })
    }
}

impl From<String> for StateKey {
    fn from(inner: String) -> Self {
        Self { key: inner }
    }
}

impl From<&'static str> for StateKey {
    fn from(s: &'static str) -> Self {
        Self { key: s.to_string() }
    }
}

pub struct BlockChainState {
    mempool: Arc<MemPool>,
    utxo: Arc<UTXO>,
    block_storage: Arc<BlockStorage>,
    state: Arc<BlockChainStateKV>,
}

unsafe impl Send for BlockChainState {}
unsafe impl Sync for BlockChainState {}

impl BlockChainState {
    pub fn new(
        mempool: Arc<MemPool>,
        utxo: Arc<UTXO>,
        block_storage: Arc<BlockStorage>,
        state: Arc<BlockChainStateKV>,
    ) -> Self {
        Self {
            mempool,
            utxo,
            block_storage,
            state,
        }
    }
}

impl KVEntry for BlockChainState {
    type Key = StateKey;
    type Value = StateValue;

    fn column() -> &'static str {
        "chain_state"
    }
}

impl BlockChainState {
    pub fn get_current_head(&self) -> Result<Option<BlockHeader>> {
        match self.state.get(&CURRENT_HEAD_KEY.into())? {
            None => Ok(None),
            Some(StateValue::CurrentHead(head)) => Ok(Some(head)),
        }
    }
    pub fn get_mempool(&self) -> Result<MempoolSnapsot> {
        self.mempool.snapshot()
    }

    fn resolve_current_head(&self, new_block: &Block) -> Result<StateEffect> {
        let current_head = self.get_current_head()?;
        match current_head {
            None => {
                self.state.put(
                    CURRENT_HEAD_KEY.into(),
                    StateValue::CurrentHead(new_block.header()),
                )?;
                return Ok(StateEffect::CurrentHeadChanged);
            }
            Some(current_head) => {
                if current_head.block_hash() == new_block.prev_block_hash() {
                    self.state.put(
                        CURRENT_HEAD_KEY.into(),
                        StateValue::CurrentHead(new_block.header()),
                    )?;
                    return Ok(StateEffect::CurrentHeadChanged);
                }
            }
        }

        Ok(StateEffect::None)
    }
    fn dispatch(&self, action: StateAction) -> Result<StateEffect> {
        return match action {
            StateAction::AddNewBlock(block) => {
                validate_block(&block)?;
                for tx_hash in block.transactions().iter() {
                    let tx = self
                        .mempool
                        .get_tx(tx_hash)?
                        .ok_or(BlockChainError::TransactionNotFound)?;
                    execute_tx(tx, self.utxo.as_ref())?;
                    self.mempool.remove(tx_hash)?;
                }

                self.block_storage.put_block(block.clone())?;
                self.resolve_current_head(&block)
            }
            StateAction::AddNewTransaction(tx) => {
                self.mempool.put(&tx)?;
                Ok(StateEffect::TxAdded)
            }
        };
    }
}

pub enum StateAction {
    AddNewBlock(Block),
    AddNewTransaction(Tx),
}

pub enum StateEffect {
    CurrentHeadChanged,
    TxAdded,
    None,
}
