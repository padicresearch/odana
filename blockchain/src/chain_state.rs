use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::{bail, Result};
use lru::LruCache;
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::H256;
use state::State;
use storage::{KVStore, Schema};
use tracing::{error, info};
use traits::{Blockchain, ChainReader, Consensus, StateDB};
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::events::LocalEventMessage;
use types::{ChainStateValue, Hash};

use crate::block_storage::BlockStorage;
use crate::errors::BlockChainError;

pub type ChainStateStorageKV = dyn KVStore<ChainStateStorage> + Send + Sync;

pub struct ChainStateStorage {
    kv: Arc<ChainStateStorageKV>,
}

impl Schema for ChainStateStorage {
    type Key = String;
    type Value = ChainStateValue;

    fn column() -> &'static str {
        "chain_state"
    }
}

const CURR_HEAD: &'static str = "ch";

impl ChainStateStorage {
    pub fn new(kv: Arc<ChainStateStorageKV>) -> Self {
        Self { kv }
    }
    pub fn set_current_header(&self, header: BlockHeader) -> Result<()> {
        self.kv.put(
            CURR_HEAD.to_string(),
            ChainStateValue::CurrentHeader(header),
        )
    }

    pub fn get_current_header(&self) -> Result<Option<BlockHeader>> {
        let value = self.kv.get(&CURR_HEAD.to_string())?;

        let r = match value {
            None => None,
            Some(ch) => {
                if let ChainStateValue::CurrentHeader(header) = ch {
                    Some(header)
                } else {
                    None
                }
            }
        };
        Ok(r)
    }
}

pub struct ChainState {
    state: Arc<State>,
    block_storage: Arc<BlockStorage>,
    chain_state: Arc<ChainStateStorage>,
    sender: UnboundedSender<LocalEventMessage>,
}

const CURR_STATE_ROOT: Hash = [0; 32];

impl ChainState {
    pub fn new(
        state_dir: PathBuf,
        consensus: Arc<dyn Consensus>,
        block_storage: Arc<BlockStorage>,
        chain_state_storage: Arc<ChainStateStorage>,
        sender: UnboundedSender<LocalEventMessage>,
    ) -> Result<Self> {
        let state = Arc::new(State::new(
            state_dir,
        )?);
        if let Some(current_head) = chain_state_storage.get_current_header()? {
            info!(current_head = ?current_head, "restore from blockchain state");
        } else {
            let genesis = consensus.get_genesis_header();
            let block = Block::new(genesis.clone(), vec![]);
            block_storage.put(block)?;
            chain_state_storage.set_current_header(genesis)?;
            info!(current_head = ?genesis, "blockchain state started from genesis");
        }

        Ok(Self {
            state,
            block_storage,
            chain_state: chain_state_storage,
            sender,
        })
    }

    pub fn put_chain(&self, consensus: Arc<dyn Consensus>, blocks: Vec<Block>) -> Result<()> {
        for block in blocks {
            match self
                .update_chain(consensus.clone(), block.clone())
                .map(|block| {
                    let header = block.header().clone();
                    header
                }) {
                Ok(header) => {
                    self.chain_state.set_current_header(header.clone())?;
                    self.sender.send(LocalEventMessage::StateChanged {
                        current_head: self.current_header().unwrap().unwrap().raw,
                    });
                    info!(header = ?H256::from(header.hash()), level = header.level, parent_hash = ?format!("{}", H256::from(header.parent_hash)), "Applied new block");
                }
                Err(e) => {
                    error!(header = ?H256::from(block.hash()), parent_hash = ?format!("{}", H256::from(block.parent_hash())), level = block.level(), error = ?e, "Error updating chain state")
                    // Todo: clean up opened states
                }
            }
        }
        Ok(())
    }

    fn update_chain(
        &self,
        consensus: Arc<dyn Consensus>,
        block: Block,
    ) -> Result<Block> {
        let mut header = block.header().clone();
        consensus.prepare_header(self.block_storage.clone(), &mut header)?;
        consensus.finalize(
            self.block_storage.clone(),
            &mut header,
            self.state.clone(),
            block.transactions().clone(),
        )?;
        consensus.verify_header(self.block_storage.clone(), &header)?;
        if header.hash() != block.hash() {
            return Err(BlockChainError::InvalidBlock.into());
        }
        Ok(block)
    }

    pub fn block_storage(&self) -> Arc<BlockStorage> {
        self.block_storage.clone()
    }

    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }
}

impl Blockchain for ChainState {
    fn get_current_state(&self) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state())
    }

    fn current_header(&self) -> anyhow::Result<Option<IndexedBlockHeader>> {
        self.chain_state
            .get_current_header()
            .map(|header| header.map(|header| header.into()))
    }

    fn get_state_at(&self, root: &Hash) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state.get_sate_at(H256::from(root))?)
    }
}

impl ChainReader for ChainState {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>> {
        self.block_storage.get_block(hash, level)
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        self.block_storage.get_block_by_hash(hash)
    }

    fn get_block_by_level(&self, level: i32) -> Result<Option<Block>> {
        self.block_storage.get_block_by_level(level)
    }
}
