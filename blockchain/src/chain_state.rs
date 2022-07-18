use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::Mutex;

use anyhow::{anyhow, bail, Result};
use lru::LruCache;
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::{H160, H256};
use state::State;
use storage::{KVStore, Schema};
use tracing::{error, info, trace, debug};
use traits::{Blockchain, ChainHeadReader, ChainReader, Consensus, StateDB};
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
    lock: RwLock<()>,
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
        let state = Arc::new(State::new(state_dir)?);
        if let Some(current_head) = chain_state_storage.get_current_header()? {
            info!(current_head = ?current_head, "restore from blockchain state");
        } else {
            let mut genesis = consensus.get_genesis_header();
            state.credit_balance(&H160::from(&[0; 20]), 1_000_000_000_000);
            state.commit()?;
            genesis.state_root = state.root();
            let block = Block::new(genesis.clone(), vec![]);
            block_storage.put(block)?;
            chain_state_storage.set_current_header(genesis)?;
            info!(current_head = ?genesis, "blockchain state started from genesis");
        }

        Ok(Self {
            lock: Default::default(),
            state,
            block_storage,
            chain_state: chain_state_storage,
            sender,
        })
    }

    pub fn put_chain(&self, consensus: Arc<dyn Consensus>, blocks: Box<dyn Iterator<Item=Block>>) -> Result<()> {
        let _ = self.lock.write().map_err(|e| anyhow!("{}", e))?;
        let mut blocks = blocks;


        for block in blocks {
            let header = block.header().clone();
            match self
                .process_block(consensus.clone(), block)
                .and_then(|block| self.accept_block(consensus.clone(), block))
            {
                Ok(_) => {}
                Err(e) => {
                    trace!(header = ?H256::from(header.hash()), parent_hash = ?format!("{}", H256::from(header.parent_hash)), level = header.level, error = ?e, "Error updating chain state");
                    return Err(e);
                }
            };
        }
        Ok(())
    }

    fn process_block(&self, consensus: Arc<dyn Consensus>, block: Block) -> Result<Block> {
        let mut header = block.header().clone();
        consensus.prepare_header(self.block_storage.clone(), &mut header)?;
        let block_storage = self.block_storage();
        let parent_header = block_storage
            .get_header_by_hash(block.parent_hash())?
            .ok_or(anyhow!("error processing block parent block not found"))?;
        let parent_state_root = H256::from(parent_header.raw.state_root);
        let parent_state = self.state.get_sate_at(parent_state_root)?;
        consensus.finalize(
            self.block_storage.clone(),
            &mut header,
            parent_state,
            block.transactions().clone(),
        )?;
        consensus.verify_header(self.block_storage.clone(), &header)?;
        if header.hash() != block.hash() {
            return Err(BlockChainError::InvalidBlock.into());
        }
        Ok(block)
    }

    fn accept_block(&self, consensus: Arc<dyn Consensus>, block: Block) -> Result<()> {
        let current_head = self.current_header()?;
        let current_head =
            current_head.ok_or(anyhow!("failed to load current head, state invalid"))?;
        let header = block.header();
        if block.parent_hash() == current_head.hash.as_fixed_bytes() {
            let state = self.state();
            state.apply_txs(block.transactions().clone());
            let next_state_root = state.credit_balance(
                &H160::from(header.coinbase),
                consensus.miner_reward(header.level),
            )?;

            state.commit()?;
            self.chain_state.set_current_header(header.clone())?;
            self.sender.send(LocalEventMessage::StateChanged {
                current_head: self.current_header().unwrap().unwrap().raw,
            });
            info!(header = ?H256::from(header.hash()), level = header.level, parent_hash = ?format!("{}", H256::from(header.parent_hash)), "Applied new block");
        } else {
            let state = self.state();
            let block_storage = self.block_storage();
            // TODO: make it better
            let parent_header = block_storage
                .get_header_by_hash(block.parent_hash())?
                .ok_or(anyhow!("error accepting block non commit"))?;
            let parent_state_root = H256::from(parent_header.raw.state_root);
            state.apply_txs_no_commit(
                parent_state_root,
                consensus.miner_reward(block.level()),
                H160::from(block.header().coinbase),
                block.transactions().clone(),
            )?;
            info!(header = ?H256::from(header.hash()), level = header.level, parent_hash = ?format!("{}", H256::from(header.parent_hash)), "Accepted block No Commit");
            if block.level() > current_head.raw.level {
                debug!(header = ?H256::from(header.hash()), level = header.level, parent_hash = ?format!("{}", H256::from(header.parent_hash)), "Resetting state");
                self.state.reset(H256::from(header.state_root))?;
                self.chain_state.set_current_header(header.clone())?;
                info!(header = ?H256::from(header.hash()), level = header.level, parent_hash = ?format!("{}", H256::from(header.parent_hash)), "Chain changed, network fork");
            }
        }

        Ok(())
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
        let _ = self.lock.read().map_err(|e| anyhow!("{}", e))?;
        self.chain_state
            .get_current_header()
            .map(|header| header.map(|header| header.into()))
    }

    fn get_state_at(&self, root: &Hash) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state.get_sate_at(H256::from(root))?)
    }

    fn put_chain(&self, consensus: Arc<dyn Consensus>, blocks: Vec<Block>) -> Result<()> {
        self.put_chain(consensus, Box::new(blocks.into_iter()))
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
