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
use std::intrinsics::fabsf32;
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

impl PartialEq for OrderedBlock {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash().eq(&other.0.hash())
    }
}

impl AsRef<Block> for OrderedBlock {
    fn as_ref(&self) -> &Block {
        &self.0
    }
}

struct SyncManager {
    chain: Arc<ChainState>,
    consensus: Arc<dyn Consensus>,
    block_storage: Arc<BlockStorage>,
    sync_mode: SyncMode,
    last_request_index: i32,
    last_tip_before_sync: Option<(String, BlockHeader)>,
}

impl Actor for SyncManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(10), |_| {});
    }
}

impl Handler<KPeerMessage> for SyncManager {
    type Result = ();


    fn handle(&mut self, msg: KPeerMessage, ctx: &mut Self::Context) -> Self::Result {
        let mode = self.sync_mode.clone();
        match msg.as_ref() {
            PeerMessage::Blocks(msg) => {
                // TODO: Clean up very rough work
                let mut common_ancestor: Option<[u8; 32]> = None;
                let block_count = msg.blocks.len();
                for block in msg.blocks {
                    match self
                        .block_storage
                        .get_block(block.parent_hash(), block.level() - 1)
                    {
                        Ok(Some(block)) => common_ancestor = Some(block.hash()),
                        _ => {}
                    };
                    self.block_storage.put(block.clone()).unwrap();
                }

                match mode {
                    SyncMode::Forward => {
                        if let Some(common_ancestor) = common_ancestor {
                            self.sync_mode = SyncMode::Forward;
                            let (_, last_tip) = self.last_tip_before_sync.unwrap();
                            // Gather all block before tip and apply
                            let mut ordered_blocks = BTreeSet::new();
                            let mut cursor = last_tip.hash();

                            if block_count > 0 {
                                self.sync_mode = SyncMode::Normal;
                            }

                            loop {
                                let curr_block = match self.block_storage.get_block_by_hash(&cursor)
                                {
                                    Ok(Some(block)) => block,
                                    _ => break,
                                };
                                cursor = *curr_block.parent_hash();
                                ordered_blocks.insert(OrderedBlock(curr_block));
                                if cursor == common_ancestor {
                                    break;
                                }
                            }
                            self.last_request_index = self.last_request_index.saturating_add(24);
                            self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                                self.last_request_index,
                                24,
                            )))
                        } else {
                            self.sync_mode = SyncMode::Backward;
                            self.last_request_index = self.last_request_index.saturating_sub(24);
                            self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                                self.last_request_index,
                                24,
                            )))
                        }
                    }
                    SyncMode::Backward => {
                        if let Some(common_ancestor) = common_ancestor {
                            let (_, last_tip) = self.last_tip_before_sync.unwrap();
                            // Gather all block before tip and apply
                            let mut ordered_blocks = BTreeSet::new();
                            let mut cursor = last_tip.hash();

                            loop {
                                let curr_block = match self.block_storage.get_block_by_hash(&cursor)
                                {
                                    Ok(Some(block)) => block,
                                    _ => break,
                                };
                                cursor = *curr_block.parent_hash();
                                ordered_blocks.insert(OrderedBlock(curr_block));
                                if cursor == common_ancestor {
                                    break;
                                }
                            }

                            self.chain
                                .put_chain(
                                    self.consensus.clone(),
                                    Box::new(
                                        ordered_blocks.into_iter().map(|ob| ob.as_ref().clone()),
                                    ),
                                )
                                .unwrap()
                        } else {
                            self.last_request_index = self.last_request_index.saturating_sub(24);
                            self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                                self.last_request_index,
                                24,
                            )))
                        }
                    }
                    SyncMode::Normal => {}
                }
            }
            _ => {}
        }
    }
}

impl Handler<KLocalMessage> for SyncManager {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: KLocalMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg.as_ref() {
            LocalEventMessage::NetworkHighestHeadChanged { peer_id, tip } => {
                let local_tip = self.local_tip()?;
                if tip.level > local_tip.level {
                    //TODO stop miner
                    if self.last_tip_before_sync.is_none() {
                        self.send_peer_message(PeerMessage::FindBlocks(FindBlocksMessage::new(
                            local_tip.level,
                            24,
                        )))
                            .unwrap();
                        self.last_request_index = local_tip.level;
                        self.last_tip_before_sync = Some((peer_id.clone(), local_tip))
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl SyncManager {
    fn local_tip(&self) -> Result<BlockHeader> {
        self.chain.current_header().map(|head| head.unwrap().raw)
    }

    fn can_apply_blocks(&self, blocks: &[Block]) -> bool {
        if blocks.is_empty() {
            return false;
        }
        let pblock = &blocks[0];
        return match self
            .block_storage
            .get_block(pblock.parent_hash(), pblock.level() - 1)
        {
            Ok(block) => block.is_some(),
            Err(_) => false,
        };
    }

    // fn pending_chain_valid(&mut self) -> bool {
    //     if self.blocks_to_apply.is_empty() {
    //         return false;
    //     }
    //     let highest_block_level = self
    //         .blocks_to_apply
    //         .last_entry()
    //         .map(|entry| *entry.key())
    //         .unwrap_or_default();
    //     let lowest_block_level = self
    //         .blocks_to_apply
    //         .first_entry()
    //         .map(|entry| *entry.key())
    //         .unwrap_or_default();
    //
    //     // Make sure we have a continuous chain
    //     'l: for level in highest_block_level..lowest_block_level {
    //         let blocks = match self.blocks_to_apply.get(&level) {
    //             None => return false,
    //             Some(blocks) => blocks,
    //         };
    //         let prev_blocks = match self.blocks_to_apply.get(&(level - 1)) {
    //             None => return false,
    //             Some(blocks) => blocks,
    //         };
    //
    //         let mut t = false;
    //         for (_,b) in blocks.iter() {
    //             t |= prev_blocks.contains_key(b.parent_hash());
    //             if t  {
    //                 continue 'l;
    //             }
    //         }
    //         if !t {
    //             break
    //         }
    //     };
    //
    //     match self
    //         .block_storage
    //         .get_block(start_block.parent_hash(), start_block.level() - 1)
    //     {
    //         Ok(block) => block.is_some(),
    //         Err(_) => false,
    //     }
    // }

    pub(crate) fn send_peer_message(&self, msg: PeerMessage) {
        todo!()
    }
}
