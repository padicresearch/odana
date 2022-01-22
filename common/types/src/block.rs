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
use core::cmp;
use std::io;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockHeader {
    pub parent_hash: Hash,
    pub merkle_root: Hash,
    pub state_root: Hash,
    pub mix_nonce: Hash,
    pub coinbase: Address,
    #[getter(skip)]
    pub difficulty: u32,
    pub chain_id: u32,
    pub level: i32,
    pub time: u32,
    pub nonce: u128,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        SHA256::digest(&self.encode().unwrap()).into()
    }

    pub fn difficulty(&self) -> Compact {
        Compact::from(self.difficulty)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Box<[Transaction]>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<Hash>>>,
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
        cache_hash(&self.hash, || {
            self.header.hash()
        })
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }
    pub fn level(&self) -> i32 {
        self.header.level
    }
    pub fn parent_hash(&self) -> &Hash {
        &self.header.parent_hash
    }
}


#[derive(Clone)]
pub struct IndexedBlockHeader {
    pub hash: H256,
    pub raw: BlockHeader,
}

impl std::fmt::Debug for IndexedBlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("IndexedBlockHeader")
            .field("hash", &self.hash)
            .field("raw", &self.raw)
            .finish()
    }
}

impl From<BlockHeader> for IndexedBlockHeader {
    fn from(header: BlockHeader) -> Self {
        Self::from_raw(header)
    }
}

impl IndexedBlockHeader {
    pub fn new(hash: H256, header: BlockHeader) -> Self {
        IndexedBlockHeader {
            hash,
            raw: header,
        }
    }

    /// Explicit conversion of the raw BlockHeader into IndexedBlockHeader.
    ///
    /// Hashes the contents of block header.
    pub fn from_raw(header: BlockHeader) -> Self {
        IndexedBlockHeader::new(H256::from(header.hash()), header)
    }
}

impl cmp::PartialEq for IndexedBlockHeader {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}