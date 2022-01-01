use super::*;
use tiny_keccak::Hasher;
use std::fmt::Formatter;
use anyhow::Result;
use derive_getters::Getters;
use serde::{Serialize, Deserialize};
use codec::{Encoder, Decoder};

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockHeader {
    prev_block_hash: BlockHash,
    block_hash: BlockHash,
    time: u32,
    level : i32,
    tx_count: u16,
    merkle_root: MerkleHash,
    nonce: u128,
}

impl BlockHeader {
    pub fn new(prev_block_hash: BlockHash,
               block_hash: BlockHash,
               time: u32,
               level : i32,
               tx_count: u16,
               merkle_root: MerkleHash,
               nonce: u128) -> Self {
        Self {
            prev_block_hash,
            block_hash,
            time,
            level,
            tx_count,
            merkle_root,
            nonce
        }
    }
}

impl From<&BlockHeader> for BlockHeader {
    fn from(block: &BlockHeader) -> Self {
        Self {
            prev_block_hash: block.prev_block_hash,
            block_hash: block.block_hash,
            time: block.time,
            level: block.level,
            tx_count: block.tx_count,
            merkle_root: block.merkle_root,
            nonce: block.nonce
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockTemplate {
    prev_block_hash: BlockHash,
    time: u32,
    level : i32,
    tx_count: u16,
    merkle_root: MerkleHash,
    nonce: u128,
}

impl BlockTemplate {
    pub fn new(
        level : i32,
        nonce: u128,
        prev_block_hash: BlockHash,
        time: u32,
        tx_count: u16,
        merkle_root: MerkleHash,
    ) -> Result<Self> {
        Ok(Self {
            prev_block_hash,
            time,
            level,
            tx_count,
            merkle_root,
            nonce,
        })
    }

    pub fn block_hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.prev_block_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.nonce.to_be_bytes());
        sha3.update(&self.tx_count.to_be_bytes());
        sha3.update(&self.time.to_be_bytes());
        sha3.update(&self.level.to_be_bytes());
        sha3.finalize(&mut block_hash);
        block_hash
    }
}



#[derive(Debug, Serialize, Deserialize, Clone, Getters)]
pub struct Block {
    hash: BlockHash,
    prev_block_hash: BlockHash,
    level : i32,
    time: u32,
    tx_count: u16,
    nonce: u128,
    merkle_root: MerkleHash,
    transactions: Vec<TxHash>,
}

impl Encoder for Block {}
impl Decoder for Block {}

#[derive(Debug, Getters)]
pub struct BlockView {
    block_hash: String,
    prev_block_hash: String,
    time: u32,
    tx_count: u16,
    level : i32,
    nonce: u128,
    merkle_root: String,
    transactions: Vec<String>,
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let view = BlockView {
            block_hash: hex::encode(self.hash),
            prev_block_hash: hex::encode(self.prev_block_hash),
            time: self.time,
            tx_count: self.tx_count,
            level: self.level,
            nonce: self.nonce,
            merkle_root: hex::encode(self.merkle_root),
            transactions: self.transactions.iter().map(|tx| hex::encode(tx)).collect(),
        };
        write!(f, "{:#?}", view)
    }
}

impl Block {
    pub fn new(template: BlockTemplate, transactions: Vec<TxHash>) -> Self {
        Self {
            hash: template.block_hash(),
            prev_block_hash: template.prev_block_hash,
            level: template.level,
            time: template.time,
            tx_count: template.tx_count,
            nonce: template.nonce,
            merkle_root: template.merkle_root,
            transactions,
        }
    }

    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.prev_block_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.nonce.to_be_bytes());
        sha3.update(&self.tx_count.to_be_bytes());
        sha3.update(&self.time.to_be_bytes());
        sha3.update(&self.level.to_be_bytes());
        sha3.finalize(&mut block_hash);
        block_hash
    }

    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            prev_block_hash: self.prev_block_hash,
            block_hash: self.hash,
            time: self.time,
            level: self.level,
            tx_count: self.tx_count,
            merkle_root: self.merkle_root,
            nonce: self.nonce,
        }
    }
}
