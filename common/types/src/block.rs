use core::cmp;
use std::cmp::Ordering;
use std::fmt::Formatter;
use std::io;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use codec::impl_codec;
use codec::{Decoder, Encoder};
use crypto::SHA256;
use primitive_types::{Compact, H256, U128, U256};

use crate::tx::SignedTransaction;

use super::*;

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug, Serialize, Deserialize)]
pub struct BlockPrimaryKey(pub Hash, pub i32);

impl Encoder for BlockPrimaryKey {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::with_capacity(36);
        encoded.extend(self.1.to_be_bytes());
        encoded.extend(self.0.iter());

        return Ok(encoded);
    }
    fn encoded_size(&self) -> Result<u64> {
        return Ok(36);
    }
}

impl Decoder for BlockPrimaryKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut level: [u8; 4] = [0; 4];
        level.copy_from_slice(&buf[..4]);
        let level = i32::from_be_bytes(level);
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&buf[4..]);
        Ok(Self(hash, level))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Getters)]
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

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Getters)]
pub struct BlockHeaderHexFormat {
    pub parent_hash: H256,
    pub merkle_root: H256,
    pub state_root: H256,
    pub mix_nonce: U256,
    pub coinbase: H160,
    pub difficulty: U128,
    pub chain_id: U128,
    pub level: U128,
    pub time: U128,
    pub nonce: U128,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        SHA256::digest(&self.encode().unwrap()).into()
    }

    pub fn difficulty(&self) -> Compact {
        Compact::from(self.difficulty)
    }

    pub fn to_hex_format(&self) -> BlockHeaderHexFormat {
        BlockHeaderHexFormat {
            parent_hash: H256::from(self.parent_hash),
            merkle_root: H256::from(self.merkle_root),
            state_root: H256::from(self.state_root),
            mix_nonce: U256::from(self.mix_nonce),
            coinbase: H160::from(self.coinbase),
            difficulty: self.difficulty.into(),
            chain_id: self.chain_id.into(),
            level: (self.level as u32).into(),
            time: self.time.into(),
            nonce: self.nonce.into(),
        }
    }
}

impl std::fmt::Debug for BlockHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_hex_format().fmt(f)
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<SignedTransaction>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<Hash>>>,
}

impl Block {
    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
    }
}

impl_codec!(Block);
impl_codec!(BlockHeader);

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.hash().eq(&other.hash())
    }
}

impl Eq for Block {}

impl PartialEq for BlockHeader {
    fn eq(&self, other: &Self) -> bool {
        self.hash().eq(&other.hash())
    }
}

impl Eq for BlockHeader {}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<SignedTransaction>) -> Self {
        Self {
            header,
            transactions,
            hash: Arc::new(Default::default()),
        }
    }

    pub fn hash(&self) -> Hash {
        cache(&self.hash, || self.header.hash())
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
        IndexedBlockHeader { hash, raw: header }
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

pub struct HeightSortedBlockHeader(pub BlockHeader);

impl AsRef<BlockHeader> for HeightSortedBlockHeader {
    fn as_ref(&self) -> &BlockHeader {
        &self.0
    }
}

impl HeightSortedBlockHeader {
    pub fn hash(&self) -> Hash {
        self.0.hash()
    }
}

impl PartialEq for HeightSortedBlockHeader {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for HeightSortedBlockHeader {}

impl PartialOrd for HeightSortedBlockHeader {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.level.cmp(&other.0.level))
    }
}

impl Ord for HeightSortedBlockHeader {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.level.cmp(&other.0.level)
    }
}
