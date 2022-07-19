use crate::messages::{KLocalMessage, KPeerMessage};
use actix::prelude::*;
use anyhow::{anyhow, Result};
use blockchain::block_storage::BlockStorage;
use blockchain::chain_state::ChainState;
use p2p::message::{FindBlocksMessage, PeerMessage};
use primitive_types::H256;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::path::Component::Normal;
use std::sync::Arc;
use std::time::Duration;
use tracing::tracing_subscriber::reload::Handle;
use traits::{Blockchain, ChainReader, Consensus};
use types::block::{Block, BlockHeader};
use types::events::LocalEventMessage;

struct HeadersStore {
    data: BTreeMap<Vec<u8>, BlockHeader>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SyncMode {
    Forward,
    Backward,
    Normal,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Normal
    }
}

pub(crate) struct OrderedBlock(Block);

impl PartialOrd for OrderedBlock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.level().partial_cmp(&other.0.level())
    }
}

impl Ord for OrderedBlock {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.level().cmp(&other.0.level())
    }
}

impl PartialEq for OrderedBlock {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash().eq(&other.0.hash())
    }
}

impl Eq for OrderedBlock {}

impl AsRef<Block> for OrderedBlock {
    fn as_ref(&self) -> &Block {
        &self.0
    }
}

pub struct SyncManager {
    chain: Arc<ChainState>,
    consensus: Arc<dyn Consensus>,
    block_storage: Arc<BlockStorage>,
    sync_mode: Arc<SyncMode>,
    last_request_index: u32,
    network_tip: BlockHeader,
    last_tip_before_sync: Option<(String, BlockHeader)>,
}

impl SyncManager {
    pub fn handle_peer(&mut self, msg: PeerMessage) {
        println!("Handle Message {:#?}", msg);
        if let PeerMessage::Blocks(msg) = msg {
            let blocks_to_import = &msg.blocks;

            if blocks_to_import.is_empty() {
                self.sync_mode = Arc::new(SyncMode::Normal);
                return;
            }

            if !self.validate_chain(blocks_to_import) {
                println!("Chain validation failed");
                return;
            }

            let ordered_blocks: BTreeSet<_> = msg
                .blocks
                .clone()
                .into_iter()
                .map(|block| OrderedBlock(block))
                .collect();
            let start_block = ordered_blocks.first().unwrap();
            let has_common_ancestor = self
                .block_storage
                .get_block_by_hash(start_block.as_ref().parent_hash())
                .map_or_else(|_| false, |block| block.is_some());
            if has_common_ancestor {
                self.chain
                    .put_chain(
                        self.consensus.clone(),
                        Box::new(ordered_blocks.into_iter().map(|ob| ob.0)),
                    )
                    .unwrap();
                self.sync_mode = Arc::new(SyncMode::Forward);
                let node_head = self.chain.current_header().unwrap();
                let node_level = node_head.map(|block| block.raw.level).unwrap();
                if self.network_tip.level > node_level {
                    self.last_request_index = node_level as u32;
                    self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                        self.last_request_index as i32,
                        24,
                    )));
                } else if self.network_tip.level < node_level {} else if self.network_tip.level == node_level {}
            } else {
                self.sync_mode = Arc::new(SyncMode::Backward);

                if self.last_request_index == 0 {
                    // If we are already at zero, lets give up
                    return;
                }
                self.last_request_index = self.last_request_index.saturating_sub(24);
                self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                    self.last_request_index as i32,
                    24,
                )));
            }
        }
    }
}

impl SyncManager {
    pub fn handle_local(&mut self, msg: LocalEventMessage) -> Result<()> {
        println!("Handle Message {:#?}", msg);
        match msg {
            LocalEventMessage::NetworkHighestHeadChanged { peer_id, tip } => {
                let node_height = self.chain.current_header()?;
                let node_height = node_height.map(|block| block.raw.level).unwrap();
                self.network_tip = tip.clone();
                if self.last_tip_before_sync.is_none() && tip.level > node_height {
                    // TODO; stop mining
                    self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                        node_height,
                        24,
                    )));
                    self.last_request_index = tip.level as u32;
                    self.last_tip_before_sync = Some((peer_id.clone(), tip.clone()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl SyncManager {
    pub fn new(
        chain: Arc<ChainState>,
        consensus: Arc<dyn Consensus>,
        block_storage: Arc<BlockStorage>,
        sync_mode: Arc<SyncMode>,
    ) -> Self {
        let node_height = chain.current_header().unwrap();
        let node_height = node_height.map(|block| block.raw.level).unwrap();
        let network_tip = consensus.get_genesis_header();
        Self {
            chain,
            consensus,
            block_storage,
            sync_mode,
            last_request_index: node_height as u32,
            network_tip,
            last_tip_before_sync: None,
        }
    }

    fn local_tip(&self) -> Result<BlockHeader> {
        self.chain.current_header().map(|head| head.unwrap().raw)
    }

    fn validate_chain(&self, blocks: &[Block]) -> bool {
        let mut blocks_to_apply: BTreeMap<i32, HashMap<[u8; 32], &Block>> = BTreeMap::new();
        for block in blocks {
            let mut map = blocks_to_apply
                .entry(block.level())
                .or_insert(HashMap::new());
            map.insert(block.hash(), block);
        }

        if blocks_to_apply.len() <= 1 {
            return true;
        }

        let highest_block_level = blocks_to_apply
            .last_entry()
            .map(|entry| *entry.key())
            .unwrap_or_default();
        let lowest_block_level = blocks_to_apply
            .first_entry()
            .map(|entry| *entry.key())
            .unwrap_or_default();

        // Make sure we have a continuous chain
        'l: for level in highest_block_level..lowest_block_level {
            let blocks = match blocks_to_apply.get(&level) {
                None => return false,
                Some(blocks) => blocks,
            };
            let prev_blocks = match blocks_to_apply.get(&(level - 1)) {
                None => return false,
                Some(blocks) => blocks,
            };

            let mut t = false;
            for (_, b) in blocks.iter() {
                t |= prev_blocks.contains_key(b.parent_hash());
                if t {
                    continue 'l;
                }
            }
            if !t {
                break;
            }
        }

        true
    }

    pub(crate) fn send_peer_message(&self, msg: PeerMessage) {
        todo!()
    }
}
