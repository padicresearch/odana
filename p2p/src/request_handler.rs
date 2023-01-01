use std::sync::Arc;

use anyhow::Result;

use blockchain::blockchain::Chain;
use primitive_types::H256;
use traits::{Blockchain, ChainHeadReader, ChainReader};

use crate::message::{BlockHeaderMessage, BlocksMessage, CurrentHeadMessage, Msg};
use crate::{NetworkState, PeerId};

pub struct RequestHandler {
    blockchain: Arc<Chain>,
    network_state: Arc<NetworkState>,
}

impl RequestHandler {
    pub fn new(blockchain: Arc<Chain>, network_state: Arc<NetworkState>) -> Self {
        Self {
            blockchain,
            network_state,
        }
    }
    pub fn handle(&self, peer_id: &PeerId, request: &Msg) -> Result<Option<Msg>> {
        //TODO: Block un connected peers from requesting
        match request {
            Msg::GetCurrentHead(_) => {
                let blockchain = self.blockchain.clone();
                if let Ok(Some(current_head)) = blockchain.chain_state().current_header() {
                    return Ok(Some(Msg::CurrentHead(CurrentHeadMessage::new(
                        current_head.raw,
                    ))));
                }
                Ok(None)
            }
            Msg::GetBlockHeader(msg) => {
                let blockchain = self.blockchain.clone();
                let mut headers = Vec::with_capacity(2000);
                let res = blockchain
                    .chain_state()
                    .block_storage()
                    .get_block_by_hash(&msg.from);
                let mut level = match res {
                    Ok(Some(block)) => block.level(),
                    _ => {
                        //find common block
                        let peer_current_state =
                            self.network_state.get_peer_state(peer_id).unwrap();
                        let res = blockchain
                            .chain_state()
                            .block_storage()
                            .get_block_by_hash(peer_current_state.parent_hash());
                        match res {
                            Ok(Some(block)) => block.level(),
                            _ => {
                                let msg = Msg::BlockHeader(BlockHeaderMessage::new(headers));
                                return Ok(Some(msg));
                            }
                        }
                    }
                };
                loop {
                    let res = blockchain
                        .chain_state()
                        .block_storage()
                        .get_header_by_level(level);
                    let header = match res {
                        Ok(Some(block)) => block.raw,
                        _ => break,
                    };

                    if headers.len() >= 2000 {
                        break;
                    }

                    if Some(header.hash()) == msg.to {
                        headers.push(header);
                        break;
                    }
                    headers.push(header);
                    level += 1;
                }
                let msg = Msg::BlockHeader(BlockHeaderMessage::new(headers));
                Ok(Some(msg))
            }
            Msg::FindBlocks(msg) => {
                let res: Result<Vec<_>> = self
                    .blockchain
                    .chain_state()
                    .block_storage()
                    .get_blocks(&H256::zero(), msg.from)
                    .unwrap()
                    .take(msg.limit as usize)
                    .collect();
                match res {
                    Ok(blocks) => Ok(Some(Msg::Blocks(BlocksMessage::new(blocks)))),
                    Err(_) => Ok(Some(Msg::Blocks(BlocksMessage::new(Vec::new())))),
                }
            }
            Msg::GetBlocks(msg) => {
                let blockchain = self.blockchain.clone();
                let mut blocks = Vec::with_capacity(msg.block_hashes.len());
                for hash in msg.block_hashes.iter() {
                    let res = blockchain
                        .chain_state()
                        .block_storage()
                        .get_block_by_hash(hash);
                    match res {
                        Ok(Some(block)) => blocks.push(block),
                        _ => break,
                    }
                }

                if blocks.len() != msg.block_hashes.len() {
                    blocks.clear();
                }
                Ok(Some(Msg::Blocks(BlocksMessage::new(blocks))))
            }
            _ => Ok(None),
        }
    }
}
