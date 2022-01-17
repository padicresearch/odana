use std::fmt::Formatter;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use codec::{Decoder, Encoder};
use codec::impl_codec;

use crate::tx::Transaction;

use super::*;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
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

    pub fn parent_hash(&self) -> &BlockHash {
        &self.parent_hash
    }
    pub fn block_hash(&self) -> &BlockHash {
        &self.block_hash
    }
    pub fn time(&self) -> &u32 {
        &self.time
    }
    pub fn level(&self) -> &i32 {
        &self.level
    }
    pub fn tx_count(&self) -> &u16 {
        &self.tx_count
    } pub fn merkle_root(&self) -> &MerkleHash {
        &self.merkle_root
    }
    pub fn state_root(&self) -> &Hash {
        &self.state_root
    }

    pub fn nonce(&self) -> &u128 {
        &self.nonce
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
    coinbase : Hash,
    merkle_root: MerkleHash,
    state_root: Hash,
    nonce: u128,
}

impl BlockTemplate {
    pub fn new(
        level: i32,
        nonce: u128,
        coinbase : Hash,
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
            coinbase,
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
        sha3.update(&self.coinbase);
        sha3.update(&self.nonce.to_be_bytes());
        sha3.update(&self.tx_count.to_be_bytes());
        sha3.update(&self.time.to_be_bytes());
        sha3.update(&self.level.to_be_bytes());
        sha3.finalize(&mut block_hash);
        block_hash
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    parent_hash: Hash,
    level: i32,
    time: u32,
    tx_count: u16,
    nonce: u128,
    merkle_root: Hash,
    coinbase : Hash,
    state_root: Hash,
    transactions: Vec<Transaction>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<TxHash>>>,
}

impl Block {
    pub fn transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

impl_codec!(Block);

impl Block {
    pub fn new(template: BlockTemplate, transactions: Vec<Transaction>) -> Self {
        Self {
            parent_hash: template.parent_hash,
            level: template.level,
            time: template.time,
            tx_count: template.tx_count,
            nonce: template.nonce,
            merkle_root: template.merkle_root,
            coinbase: template.coinbase,
            state_root: template.state_root,
            transactions,
            hash: Arc::new(Default::default()),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.parent_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.state_root);
        sha3.update(&self.coinbase);
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
            block_hash: self.hash(),
            time: self.time,
            level: self.level,
            tx_count: self.tx_count,
            merkle_root: self.merkle_root,
            state_root: self.state_root,
            nonce: self.nonce,
        }
    }

    pub fn parent_hash(&self) -> &BlockHash {
        &self.parent_hash
    }
    pub fn time(&self) -> &u32 {
        &self.time
    }
    pub fn level(&self) -> &i32 {
        &self.level
    }
    pub fn tx_count(&self) -> &u16 {
        &self.tx_count
    } pub fn merkle_root(&self) -> &MerkleHash {
        &self.merkle_root
    }
    pub fn state_root(&self) -> &Hash {
        &self.state_root
    }

    pub fn coinbase(&self) -> &Hash {
        &self.coinbase
    }

    pub fn nonce(&self) -> &u128 {
        &self.nonce
    }
}
