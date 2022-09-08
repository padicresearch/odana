use std::sync::Arc;

use anyhow::Result;

use blockchain::blockchain::Tuchain;
use primitive_types::H256;
use traits::{Blockchain, ChainHeadReader, ChainReader};

use crate::message::{BlockHeaderMessage, BlocksMessage, CurrentHeadMessage, PeerMessage};
use crate::{NetworkState, PeerId};

pub struct RequestHandler {
    blockchain: Arc<Tuchain>,
    network_state: Arc<NetworkState>,
}

impl RequestHandler {
    pub fn new(blockchain: Arc<Tuchain>, network_state: Arc<NetworkState>) -> Self {
        Self {
            blockchain,
            network_state,
        }
    }
    pub fn handle(&self, peer_id: &PeerId, request: &PeerMessage) -> Result<Option<PeerMessage>> {
        match request {
            PeerMessage::GetCurrentHead(_) => {
                let blockchain = self.blockchain.clone();
                if let Ok(Some(current_head)) = blockchain.chain().current_header() {
                    return Ok(Some(PeerMessage::CurrentHead(CurrentHeadMessage::new(
                        current_head.raw,
                    ))));
                }
                Ok(None)
            }
            PeerMessage::GetBlockHeader(msg) => {
                let blockchain = self.blockchain.clone();
                let mut headers = Vec::with_capacity(2000);
                let res = blockchain
                    .chain()
                    .block_storage()
                    .get_block_by_hash(&H256::from(msg.from));
                let mut level = match res {
                    Ok(Some(block)) => block.level(),
                    _ => {
                        //find common block
                        let peer_current_state =
                            self.network_state.get_peer_state(peer_id).unwrap();
                        let res = blockchain
                            .chain()
                            .block_storage()
                            .get_block_by_hash(peer_current_state.parent_hash());
                        match res {
                            Ok(Some(block)) => block.level(),
                            _ => -1,
                        }
                    }
                };
                loop {
                    let res = blockchain
                        .chain()
                        .block_storage()
                        .get_header_by_level(level);
                    let header = match res {
                        Ok(Some(block)) => block.raw,
                        _ => break,
                    };

                    if headers.len() >= 2000 {
                        break;
                    }

                    if Some(header.hash().to_fixed_bytes()) == msg.to {
                        headers.push(header);
                        break;
                    }
                    headers.push(header);
                    level += 1;
                }
                let msg = PeerMessage::BlockHeader(BlockHeaderMessage::new(headers));
                Ok(Some(msg))
            }
            PeerMessage::FindBlocks(msg) => {
                let res: Result<Vec<_>> = self
                    .blockchain
                    .chain()
                    .block_storage()
                    .get_blocks(H256::zero(), msg.from)
                    .unwrap()
                    .take(msg.limit as usize)
                    .collect();
                match res {
                    Ok(blocks) => Ok(Some(PeerMessage::Blocks(BlocksMessage::new(blocks)))),
                    Err(_) => Ok(Some(PeerMessage::Blocks(BlocksMessage::new(Vec::new())))),
                }
            }
            PeerMessage::GetBlocks(msg) => {
                let blockchain = self.blockchain.clone();
                let mut blocks = Vec::with_capacity(msg.block_hashes.len());
                for hash in msg.block_hashes.iter() {
                    let res = blockchain
                        .chain()
                        .block_storage()
                        .get_block_by_hash(&H256::from(hash));
                    match res {
                        Ok(Some(block)) => blocks.push(block),
                        _ => break,
                    }
                }

                if blocks.len() != msg.block_hashes.len() {
                    blocks.clear();
                }
                Ok(Some(PeerMessage::Blocks(BlocksMessage::new(blocks))))
            }
            _ => Ok(None),
        }
    }
}
