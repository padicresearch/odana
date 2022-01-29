use std::collections::hash_map::RandomState;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicI8};

use anyhow::Result;
use dashmap::DashMap;
use tempdir::TempDir;

use consensus::barossa::{BarossaProtocol, Network};
use miner::worker::start_worker;
use morph::Morph;
use primitive_types::H256;
use tracing::{Level, tracing_subscriber};
use traits::{Blockchain, ChainHeadReader, ChainReader, Consensus, StateDB};
use txpool::{ResetRequest, TxPool};
use txpool::tx_lookup::AccountSet;
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::events::LocalEventMessage;
use types::Hash;

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
    fn get_current_state(&self) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(&[0; 32])
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }

    fn current_header(&self) -> Result<Option<IndexedBlockHeader>> {
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.last().cloned().map(|b| b.header().clone().into());
        Ok(block)
    }
    fn get_state_at(&self, root: &Hash) -> Result<Arc<dyn StateDB>> {
        let d = self
            .states
            .get(root)
            .ok_or(anyhow::anyhow!("no state found"))
            .map(|r| r.value().clone())?;
        Ok(d)
    }
}

impl ChainReader for DummyChain {
    fn get_block(&self, hash: &Hash, level: i32) -> Result<Option<Block>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block)
    }

    fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block)
    }
}

impl ChainHeadReader for DummyChain {
    fn get_header(&self, hash: &Hash, level: i32) -> Result<Option<IndexedBlockHeader>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block.map(|b| b.header().clone().into()))
    }

    fn get_header_by_hash(&self, hash: &Hash) -> Result<Option<IndexedBlockHeader>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block.map(|b| b.header().clone().into()))
    }

    fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>> {
        let chain = self.chain.read().map_err(|e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(level as usize).map(|bloc| bloc.header().clone());
        Ok(block.map(|header| header.into()))
    }
}

#[tokio::main]
async fn main() {
    println!(
        "Retarget Interval {}",
        consensus::constants::RETARGETING_INTERVAL
    );
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();
    let (s, r) = tokio::sync::mpsc::unbounded_channel();
    let miner = account::create_account();
    let consensus = Arc::new(BarossaProtocol::new(Network::Testnet));
    let state_db_path = TempDir::new("state").unwrap();
    let morph = Arc::new(Morph::new(state_db_path.path()).unwrap());
    let chain = Arc::new(DummyChain::new(
        vec![Block::new(consensus.get_genesis_header(), Vec::new())],
        morph.clone(),
    ));

    let txpool = Arc::new(RwLock::new(
        TxPool::new(None, None, s, chain.clone()).unwrap(),
    ));
    let interrupt = Arc::new(AtomicI8::new(2));

    let (sender, mut reciever) = tokio::sync::mpsc::unbounded_channel();

    let chain_1 = chain.clone();
    let txpool_1 = txpool.clone();
    let morph_1 = morph.clone();
    let interrupt_1 = interrupt.clone();

    let chain_2 = chain.clone();
    let txpool_2 = txpool.clone();


    tokio::spawn(async move {
        start_worker(
            miner.address.clone(),
            sender,
            consensus,
            txpool_1,
            morph_1,
            chain_1,
            interrupt_1,
        );
    });
    loop {
        if let Some(message) = reciever.recv().await {
            match message {
                LocalEventMessage::MindedBlock(block) => {
                    chain_2.add(block.clone());
                    chain_2.insert_state(block.header().state_root, morph.clone());
                    let current_head = chain_2.current_header().unwrap().map(|head| {
                        head.raw
                    });
                    let new_head = block.header();

                    let mut txpool = txpool_2.write().unwrap();
                    let reset = ResetRequest::new(current_head, new_head.clone());
                    txpool.repack(AccountSet::new(), Some(reset)).unwrap();
                }
                LocalEventMessage::BroadcastTx(_) => {}
                LocalEventMessage::TxPoolPack(_) => {}
                LocalEventMessage::StateChanged { .. } => {}
            }
        }
    }
}
