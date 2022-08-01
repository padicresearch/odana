use std::path::PathBuf;

use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};

use tokio::sync::mpsc::UnboundedSender;

use primitive_types::{H160, H256};
use state::State;
use storage::{KVStore, Schema};
use tracing::{debug, info, trace, warn};
use traits::{Blockchain, ChainHeadReader, ChainReader, Consensus, StateDB};
use txpool::tx_lookup::AccountSet;
use txpool::{ResetRequest, TxPool};
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
            info!(current_head = ?current_head.hash(), level = ?current_head.level(), "restore from blockchain state");
        } else {
            let mut genesis = consensus.get_genesis_header();
            state.credit_balance(&H160::from(&[0; 20]), 1_000_000_000_000);
            state.commit()?;
            genesis.set_state_root(H256::from(state.root()));
            let block = Block::new(genesis.clone(), vec![]);
            block_storage.put(block)?;
            chain_state_storage.set_current_header(genesis)?;
            info!(current_head = ?genesis.hash(), level = ?genesis.level(), "blockchain state started from genesis");
        }

        Ok(Self {
            lock: Default::default(),
            state,
            block_storage,
            chain_state: chain_state_storage,
            sender,
        })
    }

    pub fn put_chain(
        &self,
        consensus: Arc<dyn Consensus>,
        blocks: Box<dyn Iterator<Item=Block>>,
        txpool: Arc<RwLock<TxPool>>
    ) -> Result<()> {
        let _ = self.lock.write().map_err(|e| anyhow!("{}", e))?;
        let mut blocks = blocks.peekable();
        let current_head = self.current_header()?;
        let current_head =
            current_head.ok_or(anyhow!("failed to load current head, state invalid"))?;
        let first_block = blocks.peek().unwrap();
        if first_block.parent_hash().ne(&current_head.hash)
            && current_head.raw.level() > first_block.level() - 1
        {
            // Reset header to common head
            let _header = first_block.header();
            let block_storage = self.block_storage();
            let parent_header = block_storage
                .get_header_by_hash(first_block.parent_hash())?
                .ok_or(anyhow!("error accepting block non commit"))?;

            let parent_header_raw = &parent_header.raw;
            let parent_state_root = parent_header_raw.state_root();
            debug!(header = ?parent_header_raw.hash(), level = parent_header_raw.level(), "Resetting state to");
            self.state.reset(*parent_state_root)?;
            self.chain_state
                .set_current_header(parent_header_raw.clone())?;
            info!(header = ?parent_header_raw.hash(), level = parent_header_raw.level(), "Rolled back chain to previous");
            debug!(chain_head = ?current_head.hash, chain_tail = ?parent_header.hash, level = current_head.raw.level(), "Removing stale chain");
            // Remove current chain
            {
                let block_storage = self.block_storage();
                let mut head = current_head.raw.hash();
                let mut remove_count = 0;
                loop {
                    let (next, level) = match block_storage.get_header_by_hash(&head) {
                        Ok(Some(block)) => (block.raw.parent_hash().clone(), block.raw.level()),
                        _ => break,
                    };

                    // Delete Head from storage
                    block_storage.delete(&head, level)?;
                    remove_count += 1;
                    debug!(hash = ?head,level = current_head.raw.level(), "Deleting block");
                    head = next;

                    if next.ne(&parent_header.hash) {
                        break;
                    }
                }

                warn!( staled_blocks_count = ?remove_count, "Chain ReOrg");
            }
        }


        for block in blocks {
            let header = block.header().clone();
            match self
                .process_block(consensus.clone(), block)
                .and_then(|block| self.accept_block(consensus.clone(), block))
            {
                Ok((repack, block)) => {
                    self.block_storage.put(block.clone())?;
                    if repack {
                        let mut txpool = txpool.write().map_err(|e| anyhow::anyhow!("{}",e))?;
                        txpool.repack(AccountSet::new(), Some(ResetRequest::new(Some(block.header().clone()), current_head.raw.clone())))?;
                    }
                }
                Err(e) => {
                    trace!(header = ?header.hash(), parent_hash = ?format!("{}", header.parent_hash()), level = header.level(), error = ?e, "Error updating chain state");
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
        let parent_state_root = parent_header.raw.state_root();
        let parent_state = self.state.get_sate_at(*parent_state_root)?;
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

    fn accept_block(&self, consensus: Arc<dyn Consensus>, block: Block) -> Result<(bool, Block)> {
        let current_head = self.current_header()?;
        let current_head =
            current_head.ok_or(anyhow!("failed to load current head, state invalid"))?;
        let header = block.header();
        let mut repack = false;
        if block.parent_hash().eq(&current_head.hash) {
            let state = self.state();
            state.apply_txs(block.transactions().clone())?;
            let _ = state.credit_balance(
                header.coinbase(),
                consensus.miner_reward(header.level()),
            )?;
            state.commit()?;
            self.chain_state.set_current_header(header.clone())?;
            self.sender.send(LocalEventMessage::StateChanged {
                current_head: self.current_header().unwrap().unwrap().raw,
            })?;
            info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Applied new block");
            repack = true;
        } else {
            let state = self.state();
            let block_storage = self.block_storage();
            // TODO: make it better
            let parent_header = block_storage
                .get_header_by_hash(block.parent_hash())?
                .ok_or(anyhow!("error accepting block non commit"))?;
            let parent_state_root = parent_header.raw.state_root();
            state.apply_txs_no_commit(
                *parent_state_root,
                consensus.miner_reward(block.level()),
                *block.header().coinbase(),
                block.transactions().clone(),
            )?;
            info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Accepted block No Commit");
            if block.level() > current_head.raw.level() {
                debug!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Resetting state");
                self.state.reset(*header.state_root())?;
                self.chain_state.set_current_header(header.clone())?;
                info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Chain changed, network fork");
                repack = true
            }
        }

        Ok((repack, block))
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

    fn get_state_at(&self, root: &H256) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state.get_sate_at(*root)?)
    }
}

impl ChainReader for ChainState {
    fn get_block(&self, hash: &H256, level: i32) -> Result<Option<Block>> {
        self.block_storage.get_block(hash, level)
    }

    fn get_block_by_hash(&self, hash: &H256) -> Result<Option<Block>> {
        self.block_storage.get_block_by_hash(hash)
    }

    fn get_block_by_level(&self, level: i32) -> Result<Option<Block>> {
        self.block_storage.get_block_by_level(level)
    }
}
