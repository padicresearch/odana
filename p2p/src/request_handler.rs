use std::sync::Arc;
use tokio::io::AsyncReadExt;
use traits::{Blockchain, ChainHeadReader, ChainReader};
use anyhow::Result;
use blockchain::blockchain::Tuchain;
use crate::message::{BlockHeaderMessage, BlocksMessage, CurrentHeadMessage, PeerMessage};

pub struct RequestHandler {
    blockchain: Arc<Tuchain>,
}

impl RequestHandler {
    pub fn new(blockchain: Arc<Tuchain>) -> Self {
        Self {
            blockchain
        }
    }
    pub fn handle(&self, request: &PeerMessage) -> Result<Option<PeerMessage>> {
        match request {
            PeerMessage::GetCurrentHead(_) => {
                let blockchain = self.blockchain.clone();
                if let Ok(Some(current_head)) = blockchain.chain().current_header() {
                    return Ok(Some(PeerMessage::CurrentHead(
                        CurrentHeadMessage::new(current_head.raw),
                    )))
                }
                return Ok(None)
            }
            PeerMessage::GetBlockHeader(msg) => {
                println!("Received GetBlockHeader {:?}", msg);
                let blockchain = self.blockchain.clone();
                let mut headers = Vec::with_capacity(2000);
                let res = blockchain
                    .chain()
                    .block_storage()
                    .get_block_by_hash(&msg.from);
                let mut level = match res {
                    Ok(Some(block)) => block.level(),
                    _ => -1,
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

                    if Some(header.hash()) == msg.to {
                        headers.push(header);
                        break;
                    }
                    headers.push(header);
                    level += 1;
                }
                println!("Sending BlockHeaders {:?}", headers.len());
                let msg = PeerMessage::BlockHeader(BlockHeaderMessage::new(headers));
                Ok(Some(msg))
            }
            PeerMessage::GetBlocks(msg) => {
                println!("Received Blocks {:?}", msg);
                let blockchain = self.blockchain.clone();
                let mut blocks = Vec::with_capacity(msg.block_hashes.len());
                for hash in msg.block_hashes.iter() {
                    let res =
                        blockchain.chain().block_storage().get_block_by_hash(hash);
                    match res {
                        Ok(Some(block)) => blocks.push(block),
                        _ => break,
                    }
                }

                if blocks.len() != msg.block_hashes.len() {
                    blocks.clear();
                }
                return Ok(Some(PeerMessage::Blocks(BlocksMessage::new(blocks))))
            }
            _ => {
                return Ok(None)
            }
        }
    }
}