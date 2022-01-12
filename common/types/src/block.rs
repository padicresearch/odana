use super::*;
use crate::tx::Transaction;
use anyhow::Result;
use codec::impl_codec;
use codec::{Decoder, Encoder};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use tiny_keccak::Hasher;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockHeader {
    parent_hash: BlockHash,
    block_hash: BlockHash,
    time: u32,
    level: i32,
    tx_count: u16,
    merkle_root: MerkleHash,
    state_root: Hash,
    nonce: u128,
}

impl BlockHeader {
    pub fn new(
        parent_hash: BlockHash,
        block_hash: BlockHash,
        time: u32,
        level: i32,
        tx_count: u16,
        merkle_root: MerkleHash,
        state_root: Hash,
        nonce: u128,
    ) -> Self {
        Self {
            parent_hash,
            block_hash,
            time,
            level,
            tx_count,
            merkle_root,
            state_root,
            nonce,
        }
    }
}

impl From<&BlockHeader> for BlockHeader {
    fn from(block: &BlockHeader) -> Self {
        Self {
            parent_hash: block.parent_hash,
            block_hash: block.block_hash,
            time: block.time,
            level: block.level,
            tx_count: block.tx_count,
            merkle_root: block.merkle_root,
            state_root: block.state_root,
            nonce: block.nonce,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockTemplate {
    parent_hash: BlockHash,
    time: u32,
    level: i32,
    tx_count: u16,
    merkle_root: MerkleHash,
    state_root: Hash,
    nonce: u128,
}

impl BlockTemplate {
    pub fn new(
        level: i32,
        nonce: u128,
        parent_hash: BlockHash,
        time: u32,
        tx_count: u16,
        merkle_root: MerkleHash,
        state_root: Hash,
    ) -> Result<Self> {
        Ok(Self {
            parent_hash,
            time,
            level,
            tx_count,
            merkle_root,
            state_root,
            nonce,
        })
    }

    pub fn block_hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.parent_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.state_root);
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
    parent_hash: BlockHash,
    level: i32,
    time: u32,
    tx_count: u16,
    nonce: u128,
    merkle_root: MerkleHash,
    state_root: Hash,
    #[getter(skip)]
    transactions: Vec<Transaction>,
}

impl Block {
    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

impl_codec!(Block);

// #[derive(Debug, Getters)]
// pub struct BlockView {
//     block_hash: String,
//     prev_block_hash: String,
//     time: u32,
//     tx_count: u16,
//     level: i32,
//     nonce: u128,
//     merkle_root: String,
//     transactions: Vec<String>,
// }
//
// impl std::fmt::Display for Block {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let view = BlockView {
//             block_hash: hex::encode(self.hash),
//             prev_block_hash: hex::encode(self.prev_block_hash),
//             time: self.time,
//             tx_count: self.tx_count,
//             level: self.level,
//             nonce: self.nonce,
//             merkle_root: hex::encode(self.merkle_root),
//             transactions: self.transactions.iter().map(|tx| hex::encode(tx)).collect(),
//         };
//         write!(f, "{:#?}", view)
//     }
// }

impl Block {
    pub fn new(template: BlockTemplate, transactions: Vec<Transaction>) -> Self {
        Self {
            hash: template.block_hash(),
            parent_hash: template.parent_hash,
            level: template.level,
            time: template.time,
            tx_count: template.tx_count,
            nonce: template.nonce,
            merkle_root: template.merkle_root,
            state_root: template.state_root,
            transactions,
        }
    }

    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.parent_hash);
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
            parent_hash: self.parent_hash,
            block_hash: self.hash,
            time: self.time,
            level: self.level,
            tx_count: self.tx_count,
            merkle_root: self.merkle_root,
            state_root: self.state_root,
            nonce: self.nonce,
        }
    }
}
