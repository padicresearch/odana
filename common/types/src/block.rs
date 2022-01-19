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
use primitive_types::{Compact, U256, H256};
use crypto::SHA256;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct BlockHeader {
    pub parent_hash: Hash,
    pub merkle_root: Hash,
    pub state_root: Hash,
    pub mix_nonce: Hash,
    pub coinbase: Address,
    pub difficulty: u32,
    pub chain_id: u32,
    pub level: i32,
    pub time: u32,
    pub nonce: u128,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let mut block_hash = [0_u8; 32];
        SHA256::digest(&self.encode());
        block_hash
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Box<[Transaction]>,
    #[serde(skip)]
    hash: Arc<RwLock<Hash>>,
}

impl Block {
    pub fn transactions(&self) -> &Box<[Transaction]> {
        &self.transactions
    }
}

impl_codec!(Block);
impl_codec!(BlockHeader);

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions: transactions.into_boxed_slice(),
            hash: Arc::new(Default::default()),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        cache_hash(&self.hash, self.header.hash)
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }
}
