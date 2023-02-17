use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use account::ROOT;
use anyhow::{anyhow, bail, Result};
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::{Address, H256};
use rt_vm::WasmVM;
use state::State;
use storage::{KVStore, Schema};
use tracing::{debug, info, trace, warn};
use traits::{Blockchain, ChainHeadReader, ChainReader, Consensus, StateDB};
use txpool::tx_lookup::AccountSet;
use txpool::{ResetRequest, TxPool};
use types::account::{get_address_from_package_name, AppState};
use types::app::AppStateKey;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::events::LocalEventMessage;
use types::network::Network;
use types::ChainStateValue;

use crate::block_storage::BlockStorage;
use crate::errors::BlockChainError;
use crate::errors::BlockChainError::FailedToVerifyHeader;

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

const CURR_HEAD: &str = "ch";

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
            Some(ch) => match ch {
                ChainStateValue::CurrentHeader(header) => Some(header),
            },
        };
        Ok(r)
    }
}

pub struct ChainState {
    lock: RwLock<()>,
    state: Arc<State>,
    consensus: Arc<dyn Consensus>,
    block_storage: Arc<BlockStorage>,
    chain_state: Arc<ChainStateStorage>,
    vm: Arc<WasmVM>,
    sender: UnboundedSender<LocalEventMessage>,
}

impl ChainState {
    pub fn new(
        state_dir: PathBuf,
        consensus: Arc<dyn Consensus>,
        block_storage: Arc<BlockStorage>,
        built_in: Vec<(&'static str, &[u8])>,
        chain_state_storage: Arc<ChainStateStorage>,
        sender: UnboundedSender<LocalEventMessage>,
    ) -> Result<Self> {
        let state = Arc::new(State::new(state_dir)?);
        let vm = if let Some(current_head) = chain_state_storage.get_current_header()? {
            state.reset(*current_head.state_root())?;
            let vm = Arc::new(WasmVM::new(block_storage.clone())?);
            for (pkn, binary) in built_in {
                let _ = vm.install_builtin(
                    state.clone(),
                    get_address_from_package_name(pkn, consensus.network())?,
                    binary,
                    false,
                )?;
            }
            info!(blockhash = ?current_head.hash(), level = ?current_head.level(), "restore from blockchain state");
            vm
        } else {
            // TODO: Clean up genesis generation to use a config file or function
            let mut genesis = consensus.get_genesis_header();
            state.credit_balance(&Address::default(), 1_000_000_000_000)?;
            let vm = Arc::new(WasmVM::new(block_storage.clone())?);
            let mut states = HashMap::new();
            for (pkn, binary) in built_in {
                let app_address = get_address_from_package_name(pkn, consensus.network())?;
                let changelist = vm
                    .install_builtin(state.clone(), app_address, binary, true)?
                    .ok_or(anyhow!("required app not installed"))?;
                let code_hash = crypto::keccak256(binary);
                for (addr, state) in changelist.account_changes {
                    states.insert(addr, state);
                }
                let app_state = states
                    .get_mut(&app_address)
                    .ok_or_else(|| anyhow::anyhow!("app state not found"))?;
                app_state.app_state =
                    Some(AppState::new(changelist.storage.root(), code_hash, ROOT, 1));
                state.appdata.put(
                    AppStateKey(app_address, changelist.storage.root()),
                    changelist.storage,
                )?;
            }
            for (addr, account_state) in states {
                state.set_account_state(addr, account_state)?;
            }
            state.commit()?;
            genesis.set_state_root(H256::from(state.root()));
            let block = Block::new(genesis, vec![]);
            block_storage.put(block)?;
            chain_state_storage.set_current_header(genesis)?;
            info!(blockhash = ?genesis.hash(), level = ?genesis.level(), "blockchain state started from genesis");
            vm
        };

        Ok(Self {
            lock: Default::default(),
            state,
            consensus,
            block_storage,
            chain_state: chain_state_storage,
            vm,
            sender,
        })
    }

    pub fn put_chain(
        &self,
        consensus: Arc<dyn Consensus>,
        blocks: Box<dyn Iterator<Item = Block>>,
        txpool: Arc<RwLock<TxPool>>,
    ) -> Result<()> {
        let _lock = self.lock.write().map_err(|e| anyhow!("{}", e))?;
        let mut blocks = blocks.peekable();
        let current_head = self.current_header()?;
        let current_head =
            current_head.ok_or_else(|| anyhow!("failed to load current head, state invalid"))?;
        let first_block = blocks.peek().unwrap();
        if first_block.parent_hash().ne(&current_head.hash)
            && current_head.raw.level() > first_block.level() - 1
        {
            // Reset header to common head
            let _header = first_block.header();
            let block_storage = self.block_storage();
            let parent_header = block_storage
                .get_header_by_hash(first_block.parent_hash())?
                .ok_or_else(|| anyhow!("error accepting block non commit"))?;

            let parent_header_raw = &parent_header.raw;
            let parent_state_root = parent_header_raw.state_root();
            debug!(blockhash = ?parent_header_raw.hash(), level = parent_header_raw.level(), "Resetting state to");
            self.state.reset(*parent_state_root)?;
            self.chain_state.set_current_header(*parent_header_raw)?;
            info!(blockhash = ?parent_header_raw.hash(), level = parent_header_raw.level(), "Rolled back chain to previous");
            debug!(chain_head = ?current_head.hash, chain_tail = ?parent_header.hash, level = current_head.raw.level(), "Removing stale chain");
            // Remove current chain
            {
                let block_storage = self.block_storage();
                let mut head = current_head.raw.hash();
                let mut remove_count = 0;
                loop {
                    let (next, level) = match block_storage.get_header_by_hash(&head) {
                        Ok(Some(block)) => (*block.raw.parent_hash(), block.raw.level()),
                        _ => break,
                    };

                    // Delete Head from storage
                    block_storage.delete(&head, level)?;
                    remove_count += 1;
                    debug!(blockhash = ?head,level = current_head.raw.level(), "Deleting block");
                    head = next;

                    if next.ne(&parent_header.hash) {
                        break;
                    }
                }

                warn!( staled_blocks_count = ?remove_count, "Chain ReOrg");
            }
        }

        for block in blocks {
            let header = *block.header();
            match self
                .process_block(consensus.clone(), block)
                .and_then(|block| self.accept_block(consensus.clone(), block))
            {
                Ok((repack, store_block, block)) => {
                    if store_block {
                        self.block_storage.put(block.clone())?;
                    }

                    if repack {
                        let mut txpool = txpool.write().map_err(|e| anyhow::anyhow!("{}", e))?;
                        txpool.repack(
                            AccountSet::new(),
                            Some(ResetRequest::new(Some(current_head.raw), *block.header())),
                        )?;
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

    pub fn current_header_blocking(&self) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let _lock = self.lock.read().map_err(|e| anyhow!("{}", e))?;
        self.chain_state
            .get_current_header()
            .map(|header| header.map(|header| header.into()))
    }

    pub fn try_get_current_header(&self) -> anyhow::Result<Option<IndexedBlockHeader>> {
        let _lock = self.lock.try_read().map_err(|e| anyhow!("{}", e))?;
        self.chain_state
            .get_current_header()
            .map(|header| header.map(|header| header.into()))
    }

    fn process_block(&self, consensus: Arc<dyn Consensus>, block: Block) -> Result<Block> {
        let mut header = *block.header();
        consensus.prepare_header(self.block_storage.clone(), &mut header)?;
        let block_storage = self.block_storage();
        let parent_header = block_storage
            .get_header_by_hash(block.parent_hash())?
            .ok_or_else(|| anyhow!("error processing block parent block not found"))?;
        let parent_state_root = parent_header.raw.state_root();
        let parent_state = self.state.get_sate_at(*parent_state_root)?;
        consensus.finalize(
            self.block_storage.clone(),
            &mut header,
            self.vm.clone(),
            parent_state,
            block.transactions(),
        )?;
        consensus
            .verify_header(self.block_storage.clone(), &header)
            .map_err(|e| FailedToVerifyHeader(header.into(), (*block.header()).into(), e))?;
        if header.hash() != block.hash() {
            return Err(BlockChainError::InvalidBlock.into());
        }
        Ok(block)
    }

    fn accept_block(
        &self,
        consensus: Arc<dyn Consensus>,
        block: Block,
    ) -> Result<(bool, bool, Block)> {
        let current_head = self.current_header()?;
        let current_head =
            current_head.ok_or_else(|| anyhow!("failed to load current head, state invalid"))?;
        let header = block.header();
        let mut repack = false;
        if block.parent_hash().eq(&current_head.hash) {
            let state = self.state();
            state.apply_txs(self.vm.clone(), block.transactions())?;
            let _ =
                state.credit_balance(header.coinbase(), consensus.miner_reward(header.level()))?;
            state.commit()?;
            self.chain_state.set_current_header(*header)?;
            self.sender.send(LocalEventMessage::StateChanged {
                current_head: self.current_header().unwrap().unwrap().raw,
            })?;
            info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), tx_count = block.transactions().len(), "Applied new block");
            repack = true;
        } else {
            let state = self.state();
            let block_storage = self.block_storage();
            // TODO: make it better
            let parent_header = block_storage
                .get_header_by_hash(block.parent_hash())?
                .ok_or_else(|| anyhow!("error accepting block non commit"))?;
            let parent_state_root = parent_header.raw.state_root();
            let commit_state = state.apply_txs_no_commit(
                self.vm.clone(),
                *parent_state_root,
                consensus.miner_reward(block.level()),
                *block.header().coinbase(),
                block.transactions(),
            )?;
            let commit_state = H256::from(commit_state);
            if commit_state.ne(header.state_root()) {
                warn!(header = ?header.hash(), expected_state_root = ?commit_state , block_state_root = ?header.state_root(), parent_hash = ?format!("{}", header.parent_hash()), "Rejected block with invalid state");
                bail!("Invalid or Corrupt Block")
            }

            info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Accepted block No Commit");
            if block.level() > current_head.raw.level() {
                debug!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Resetting state");
                self.state.reset(*header.state_root())?;
                self.chain_state.set_current_header(*header)?;
                info!(header = ?header.hash(), level = header.level(), parent_hash = ?format!("{}", header.parent_hash()), "Chain changed, network fork");
                repack = true
            }
        }

        Ok((repack, true, block))
    }

    pub fn block_storage(&self) -> Arc<BlockStorage> {
        self.block_storage.clone()
    }

    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }
    pub fn vm(&self) -> Arc<WasmVM> {
        self.vm.clone()
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

    fn get_state_at(&self, root: &H256) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state.get_sate_at(*root)?)
    }

    fn genesis(&self) -> IndexedBlockHeader {
        let Ok(Some(genesis)) =
            self.block_storage
                .get_header_by_level(0) else {
            panic!("genesis not found")
        };
        genesis
    }

    fn network(&self) -> Network {
        self.consensus.network()
    }
}

impl ChainReader for ChainState {
    fn get_block(&self, hash: &H256, level: u32) -> Result<Option<Block>> {
        self.block_storage.get_block(hash, level)
    }

    fn get_block_by_hash(&self, hash: &H256) -> Result<Option<Block>> {
        self.block_storage.get_block_by_hash(hash)
    }

    fn get_block_by_level(&self, level: u32) -> Result<Option<Block>> {
        self.block_storage.get_block_by_level(level)
    }
}
