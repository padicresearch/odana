use crate::error::NodeError;
use anyhow::{anyhow, ensure, Result};
use blockchain::block_storage::BlockStorage;
use blockchain::chain_state::ChainState;
use p2p::message::{BlocksMessage, FindBlocksMessage, NodeToPeerMessage, PeerMessage};
use primitive_types::H256;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::path::Component::Normal;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tracing::tracing_subscriber::reload::Handle;
use tracing::warn;
use traits::{Blockchain, ChainReader, Consensus, Handler};
use txpool::TxPool;
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

pub struct OrderedBlock(Block);

impl PartialOrd for OrderedBlock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.level().partial_cmp(&other.0.level())
    }
}

impl Ord for OrderedBlock {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.level().cmp(&other.0.level()) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.0.hash().cmp(&other.0.hash()),
            Ordering::Greater => Ordering::Greater,
        }
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

pub struct SyncService {
    chain: Arc<ChainState>,
    txpool: Arc<RwLock<TxPool>>,
    consensus: Arc<dyn Consensus>,
    block_storage: Arc<BlockStorage>,
    sync_mode: Arc<SyncMode>,
    last_request_index: u32,
    network_tip: BlockHeader,
    highest_peer: String,
    sender: Arc<UnboundedSender<NodeToPeerMessage>>,
    tip_before_sync: Option<(String, BlockHeader)>,
}

impl SyncService {
    pub fn handle_remote_message(&mut self, msg: PeerMessage) -> Result<()> {
        return match msg {
            PeerMessage::Blocks(msg) => self.handle_import_blocks(&msg),
            PeerMessage::BroadcastTransaction(_) => Ok(()),
            PeerMessage::BroadcastBlock(_) => Ok(()),
            _ => Ok(()),
        };
    }

    fn handle_import_blocks(&mut self, msg: &BlocksMessage) -> Result<()> {
        let blocks_to_import = &msg.blocks;
        let node_head = self.chain.current_header_blocking().unwrap();
        let node_level = node_head.map(|block| block.raw.level()).unwrap();

        if blocks_to_import.is_empty() && node_level >= self.network_tip.level() {
            self.sync_mode = Arc::new(SyncMode::Normal);
            return Ok(());
        }

        ensure!(
            self.validate_chain(blocks_to_import),
            NodeError::ChainValidationFailed
        );

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
            .map_or_else(|_| false, |block| block.is_some())
            || start_block.0.level() == 0;
        if has_common_ancestor {
            println!("sync chain.put_chain");
            self.chain.put_chain(
                self.consensus.clone(),
                Box::new(ordered_blocks.into_iter().map(|ob| ob.0)),
                self.txpool.clone(),
            )?;
            self.sync_mode = Arc::new(SyncMode::Forward);
            let node_head = self.chain.current_header().unwrap();
            let node_level = node_head.map(|block| block.raw.level()).unwrap();

            let (_, sync_point) = self.tip_before_sync.as_ref().unwrap();

            if sync_point.level() > node_level {
                self.last_request_index = node_level as u32 + 1;
                self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                    self.last_request_index as i32,
                    24,
                )));
            } else if sync_point.level() <= node_level {
                self.tip_before_sync = None;
                if node_level < self.network_tip.level() {
                    self.last_request_index = node_level as u32 + 1;
                    self.tip_before_sync =
                        Some((self.highest_peer.clone(), self.network_tip.clone()));
                    self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                        self.last_request_index as i32,
                        24,
                    )));
                }
            }
        } else {
            self.sync_mode = Arc::new(SyncMode::Backward);

            if self.last_request_index == 0 {
                // If we are already at zero, lets give up
                return Ok(());
            }
            self.last_request_index = self.last_request_index.saturating_sub(24);
            if self.last_request_index == 0 {
                self.last_request_index = 1
            }
            self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                self.last_request_index as i32,
                24,
            )));
        }

        Ok(())
    }
}

impl Handler<LocalEventMessage> for SyncService {
    fn handle(&mut self, msg: LocalEventMessage) {
        match self.handle_local_message(msg) {
            Ok(_) => {}
            Err(error) => {
                warn!(target: "sync", error = ?error, "failed to handle local message");
            }
        }
    }
}

impl Handler<PeerMessage> for SyncService {
    fn handle(&mut self, msg: PeerMessage) {
        match self.handle_remote_message(msg.clone()) {
            Ok(_) => {}
            Err(error) => {
                warn!(target: "sync", error = ?error, msg = format!("{:#?}", msg), "failed to handle remote message");
            }
        }
    }
}

impl SyncService {
    pub fn handle_local_message(&mut self, msg: LocalEventMessage) -> Result<()> {
        match msg {
            LocalEventMessage::NetworkHighestHeadChanged { peer_id, tip } => {
                let current_header = self.chain.current_header_blocking()?;
                let current_header = current_header.unwrap();
                let tip = tip.unwrap_or(current_header.raw.clone());
                let node_height = current_header.raw.level();
                self.network_tip = tip.clone();
                self.highest_peer = peer_id.clone();
                if self.tip_before_sync.is_none() && tip.level() > node_height {
                    // TODO; stop mining
                    self.last_request_index = tip.level() as u32;
                    self.tip_before_sync = Some((peer_id.clone(), tip.clone()));
                    self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                        node_height + 1,
                        24,
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl SyncService {
    pub fn new(
        chain: Arc<ChainState>,
        txpool: Arc<RwLock<TxPool>>,
        sender: Arc<UnboundedSender<NodeToPeerMessage>>,
        consensus: Arc<dyn Consensus>,
        block_storage: Arc<BlockStorage>,
        sync_mode: Arc<SyncMode>,
    ) -> Self {
        let node_height = chain.current_header().unwrap();
        let node_height = node_height.map(|block| block.raw.level()).unwrap();
        let network_tip = consensus.get_genesis_header();
        Self {
            chain,
            txpool,
            consensus,
            block_storage,
            sync_mode,
            last_request_index: node_height as u32,
            network_tip,
            highest_peer: "".to_string(),
            sender,
            tip_before_sync: None,
        }
    }

    fn local_tip(&self) -> Result<BlockHeader> {
        self.chain.current_header_blocking().map(|head| head.unwrap().raw)
    }

    fn validate_chain(&self, blocks: &[Block]) -> bool {
        let mut blocks_to_apply: BTreeMap<i32, HashMap<H256, &Block>> = BTreeMap::new();
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

    pub fn send_peer_message(&self, msg: PeerMessage) {
        if let Some((peer, _)) = &self.tip_before_sync {
            self.sender
                .send(NodeToPeerMessage {
                    peer_id: Some(peer.clone()),
                    message: msg,
                })
                .unwrap();
        }
    }
}
