use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Error, Result};
use lru::LruCache;
use tokio::sync::mpsc::UnboundedSender;

use morph::Morph;
use primitive_types::H256;
use storage::PersistentStorage;
use tracing::{info, warn};
use traits::{Blockchain, ChainHeadReader, ChainReader, Consensus, StateDB};
use txpool::TxPool;
use types::block::{Block, IndexedBlockHeader};
use types::events::LocalEventMessage;
use types::Hash;

use crate::block_storage::BlockStorage;
use crate::chain_state::ChainStateStorage;
use crate::errors::BlockChainError;

pub struct Tuchain {
    chain: Arc<ChainState>,
    txpool: Arc<RwLock<TxPool>>,
}

impl Tuchain {
    pub fn initialize(dir: PathBuf, consensus: Arc<dyn Consensus>, main_storage: Arc<PersistentStorage>, lmpsc: UnboundedSender<LocalEventMessage>) -> Result<Self> {
        let chain_state_storage = Arc::new(ChainStateStorage::new(main_storage.database()));
        let block_storage = Arc::new(BlockStorage::new(main_storage));
        let chain_state = Arc::new(ChainState::new(dir.join("state"), consensus.clone(), block_storage, chain_state_storage)?);
        let txpool = Arc::new(RwLock::new(TxPool::new(None, None, lmpsc, chain_state.clone())?));

        Ok(Tuchain {
            chain: chain_state,
            txpool,
        })
    }

    pub fn chain(&self) -> Arc<ChainState> {
        self.chain.clone()
    }

    pub fn txpool(&self) -> Arc<RwLock<TxPool>> {
        self.txpool.clone()
    }
}

pub struct ChainState {
    state_provider: Arc<Mutex<LruCache<Hash, Arc<Morph>>>>,
    state_dir: PathBuf,
    block_storage: Arc<BlockStorage>,
    chain_state: Arc<ChainStateStorage>,
}

const CURR_STATE_ROOT: Hash = [0; 32];

impl ChainState {
    pub fn new(
        state_dir: PathBuf,
        consensus: Arc<dyn Consensus>,
        block_storage: Arc<BlockStorage>,
        chain_state_storage: Arc<ChainStateStorage>,
    ) -> Result<Self> {
        let mut state_provider = LruCache::new(10);

        if let Some(current_head) = chain_state_storage.get_current_header()? {
            let state = Arc::new(Morph::new(
                state_dir.join(format!("{:?}", H256::from(current_head.state_root))),
            )?);
            state_provider.put(current_head.state_root, state.clone());
            state_provider.put(CURR_STATE_ROOT, state.clone());
            info!(current_head = ?current_head, "restore from blockchain state");
        } else {
            let genesis = consensus.get_genesis_header();
            let block = Block::new(genesis.clone(), vec![]);
            block_storage.put(block)?;
            chain_state_storage.set_current_header(genesis)?;
            let state = Arc::new(Morph::new(
                state_dir.join(format!("{:?}", H256::from(genesis.state_root))),
            )?);
            state_provider.put(genesis.state_root, state.clone());
            info!(current_head = ?genesis, "blockchain state started from genesis");
        }

        Ok(Self {
            state_provider: Arc::new(Mutex::new(state_provider)),
            state_dir,
            block_storage,
            chain_state: chain_state_storage,
        })
    }

    pub fn put_chain(&self, consensus: Arc<dyn Consensus>, blocks: Vec<Block>) -> Result<()> {
        for block in blocks {
            match self
                .update_chain(consensus.clone(), block)
                .and_then(|(block, state)| {
                    let header = block.header().clone();
                    self.block_storage
                        .put(block.clone())
                        .and_then(|_| Ok((header, state)))
                }) {
                Ok((header, new_state)) => {
                    let mut provider = self
                        .state_provider
                        .lock()
                        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                    provider.put(header.state_root, new_state.clone());
                    provider.put(CURR_STATE_ROOT, new_state);
                    self.chain_state.set_current_header(header)?;
                }
                Err(_) => {
                    // remove
                }
            }
        }
        Ok(())
    }

    fn update_chain(
        &self,
        consensus: Arc<dyn Consensus>,
        block: Block,
    ) -> Result<(Block, Arc<Morph>)> {
        let current_state = self.state()?;
        let state_intermediate = Arc::new(
            current_state.checkpoint(
                self.state_dir
                    .join(format!("{:?}", H256::from(block.header().state_root))),
            )?,
        );
        let mut header = block.header().clone();
        consensus.prepare_header(self.block_storage.clone(), &mut header)?;
        consensus.finalize(
            self.block_storage.clone(),
            &mut header,
            state_intermediate.clone(),
            block.transactions().clone(),
        )?;
        consensus.verify_header(self.block_storage.clone(), &header)?;
        if header.hash() != block.hash() {
            return Err(BlockChainError::InvalidBlock.into());
        }

        Ok((block, state_intermediate))
    }

    pub fn load_state(&self, root_hash: &Hash) -> Result<Arc<Morph>> {
        let mut provider = self
            .state_provider
            .lock()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        match provider.get(root_hash) {
            None => {
                let state =
                    Morph::new(self.state_dir.join(format!("{:?}", H256::from(root_hash))))?;
                return Ok(Arc::new(state));
            }
            Some(state) => Ok(state.clone()),
        }
    }

    pub fn block_storage(&self) -> Arc<BlockStorage> {
        self.block_storage.clone()
    }

    pub fn state(&self) -> anyhow::Result<Arc<Morph>> {
        let mut provider = self
            .state_provider
            .lock()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        provider
            .get(&CURR_STATE_ROOT)
            .ok_or(anyhow::anyhow!("No state found"))
            .map(|value| value.clone())
    }
}

impl Blockchain for ChainState {
    fn get_current_state(&self) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.state()?)
    }

    fn current_header(&self) -> anyhow::Result<Option<IndexedBlockHeader>> {
        self.chain_state
            .get_current_header()
            .map(|header| header.map(|header| header.into()))
    }

    fn get_state_at(&self, root: &Hash) -> anyhow::Result<Arc<dyn StateDB>> {
        Ok(self.load_state(root)?)
    }
}

impl ChainReader for ChainState {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>> {
        self.block_storage.get_block(hash, level)
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        self.block_storage.get_block_by_hash(hash)
    }
}

// impl ChainHeadReader for ChainState {
//     fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<IndexedBlockHeader>> {
//         self.block_storage.get_header(hash, level)
//     }
//
//     fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<IndexedBlockHeader>> {
//        self.block_storage.get_header_by_hash(hash)
//     }
//
//     fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>> {
//         self.block_storage.get_header_by_level(level)
//     }
// }
