use crate::worker::start_worker;
use consensus::barossa::{BarossaProtocol, Network};
use std::sync::{Arc, RwLock};
use txpool::TxPool;
use morph::Morph;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::Hash;
use tempdir::TempDir;
use traits::{Blockchain, StateDB, ChainHeadReader, Consensus};
use anyhow::Result;
use primitive_types::H256;
use dashmap::DashMap;
use std::sync::atomic::AtomicI8;

mod worker;

#[derive(Clone)]
pub struct DummyChain {
    chain: Arc<RwLock<Vec<Block>>>,
    blocks: DashMap<[u8; 32], usize>,
    states: DashMap<[u8; 32], Arc<Morph>>,
}

impl DummyChain {
    fn new(blocks: Vec<Block>, inital_state: Arc<Morph>) -> Self {
        let c: DashMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(height, block)| (block.hash(), height))
            .collect();

        let map = DashMap::new();
        map.insert([0; 32], inital_state);

        Self {
            chain: Arc::new(RwLock::new(blocks)),
            blocks: c,
            states: map,
        }
    }

    pub fn insert_state(&self, root: Hash, state: Arc<Morph>) {
        self.states.insert(root, state.clone());
        self.states.insert([0; 32], state);
    }

    pub fn add(&self, block: Block) {
        let mut chain = self.chain.write().unwrap();
        chain.push(block.clone());
        self.blocks.insert(block.hash(), chain.len() - 1);
    }
}

impl Blockchain for DummyChain {
    fn current_head(&self) -> Result<BlockHeader> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        Ok(blocks.last().map(|block| block.header().clone()).unwrap())
    }

    fn get_block(&self, block_hash: &Hash) -> Result<Option<Block>> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        let res = self
            .blocks
            .get(block_hash)
            .ok_or(anyhow::anyhow!("block not found"))?;
        let block_level = res.value().clone();
        Ok(blocks.get(block_level).cloned())
    }

    fn get_state_at(&self, root: &Hash) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(root)
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }

    fn get_current_state(&self) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(&[0; 32])
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }
}

impl ChainHeadReader for DummyChain {
    fn current_header(&self) -> Result<Option<IndexedBlockHeader>> {
        let head = self.current_head()?;
        Ok(Some(head.into()))
    }

    fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<IndexedBlockHeader>> {
        let block = self.get_block(hash)?;
        match block {
            None => {
                return Ok(None);
            }
            Some(block) => {
                Ok(Some(block.header().clone().into()))
            }
        }
    }

    fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<IndexedBlockHeader>> {
        let block = self.get_block(hash)?;
        match block {
            None => {
                return Ok(None);
            }
            Some(block) => {
                Ok(Some(block.header().clone().into()))
            }
        }
    }

    fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        let block = blocks.get(level as usize);
        todo!()
    }
}


fn main() {
    let (s, r) = tokio::sync::mpsc::unbounded_channel();
    let miner = account::create_account();
    let consensus = Arc::new(BarossaProtocol::new(Network::Testnet));
    let state_db_path = TempDir::new("state").unwrap();
    let morph = Arc::new(Morph::new(state_db_path.path()).unwrap());
    let chain = Arc::new(DummyChain::new(vec![Block::new(consensus.get_genesis_header(), Vec::new())], morph.clone()));

    let txpool = Arc::new(RwLock::new(TxPool::new(None, None, s, chain.clone()).unwrap()));
    let interrupt = Arc::new(AtomicI8::new(0));
    start_worker(chain.clone(), miner.address, consensus, txpool, morph, chain, interrupt);
    println!("{:#?}", miner)
}
