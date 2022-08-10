use core::cmp;
use std::cmp::Ordering;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use serde::{Deserialize, Serialize};

use codec::{impl_codec, ConsensusCodec};
use codec::{Decoder, Encoder};
use crypto::SHA256;
use primitive_types::{Compact, H256, U128, U256};
use proto::{Block as ProtoBlock, BlockHeader as ProtoBlockHeader};

use crate::tx::SignedTransaction;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde_json::json;

use super::*;
const HEADER_SIZE: usize = 180;

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug, Serialize, Deserialize)]
pub struct BlockPrimaryKey(pub Hash, pub i32);

impl Encoder for BlockPrimaryKey {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::with_capacity(36);
        encoded.extend(self.1.to_be_bytes());
        encoded.extend(self.0.iter());

        Ok(encoded)
    }
    fn encoded_size(&self) -> Result<u64> {
        Ok(36)
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

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Getters, Setters, MutGetters, CopyGetters)]
pub struct BlockHeader {
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    parent_hash: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    merkle_root: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    state_root: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    mix_nonce: U256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    coinbase: H160,
    #[getset(set = "pub", get_mut = "pub")]
    difficulty: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    chain_id: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    level: i32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    time: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    nonce: U128,
}
#[allow(clippy::too_many_arguments)]
impl BlockHeader {
    pub fn new(
        parent_hash: H256,
        merkle_root: H256,
        state_root: H256,
        mix_nonce: U256,
        coinbase: H160,
        difficulty: u32,
        chain_id: u32,
        level: i32,
        time: u32,
        nonce: U128,
    ) -> Self {
        Self {
            parent_hash,
            merkle_root,
            state_root,
            mix_nonce,
            coinbase,
            difficulty,
            chain_id,
            level,
            time,
            nonce,
        }
    }

    pub fn hash(&self) -> H256 {
        SHA256::digest(self.consensus_encode())
    }

    pub fn difficulty(&self) -> Compact {
        Compact::from(self.difficulty)
    }

    pub fn into_proto(self) -> Result<ProtoBlockHeader> {
        let json_rep = serde_json::to_vec(&self)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl ConsensusCodec for BlockHeader {
    fn consensus_encode(self) -> Vec<u8> {
        let mut encoded = BytesMut::with_capacity(HEADER_SIZE);
        encoded.extend(self.parent_hash.as_bytes());
        encoded.extend(self.merkle_root.as_bytes());
        encoded.extend(self.state_root.as_bytes());
        encoded.extend(self.mix_nonce.to_be_bytes());
        encoded.extend(self.coinbase.as_bytes());
        encoded.extend(self.difficulty.to_be_bytes());
        encoded.extend(self.chain_id.to_be_bytes());
        encoded.extend(self.level.to_be_bytes());
        encoded.extend(self.time.to_be_bytes());
        encoded.extend(self.nonce.to_be_bytes());
        encoded.to_vec()
    }

    fn consensus_decode(buf: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::copy_from_slice(buf);
        Ok(Self {
            parent_hash: H256::from_slice(&bytes.copy_to_bytes(32)),
            merkle_root: H256::from_slice(&bytes.copy_to_bytes(32)),
            state_root: H256::from_slice(&bytes.copy_to_bytes(32)),
            mix_nonce: U256::from_big_endian(&bytes.copy_to_bytes(32)),
            coinbase: H160::from_slice(&bytes.copy_to_bytes(20)),
            difficulty: bytes.get_u32(),
            chain_id: bytes.get_u32(),
            level: bytes.get_i32(),
            time: bytes.get_u32(),
            nonce: U128::from(bytes.get_u128()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<SignedTransaction>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<H256>>>,
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

    pub fn hash(&self) -> H256 {
        cache(&self.hash, || self.header.hash())
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }
    pub fn level(&self) -> i32 {
        self.header.level
    }
    pub fn parent_hash(&self) -> &H256 {
        self.header.parent_hash()
    }
    pub fn into_proto(self) -> Result<ProtoBlock> {
        let jsblock = json!({
            "hash" : self.hash(),
            "header" : self.header(),
            "txs" : self.transactions()
        });
        let json_rep = serde_json::to_vec(&jsblock)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
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
        IndexedBlockHeader::new(header.hash(), header)
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
    pub fn hash(&self) -> H256 {
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

#[test]
fn test_proto_conversions() {
    let block_header = BlockHeader::new(
        H256::from([1; 32]),
        H256::from([2; 32]),
        H256::from([6; 32]),
        U256::from(400),
        H160::from([7; 20]),
        30,
        30,
        30,
        10000000,
        U128::from(5),
    );

    let pheader = block_header.into_proto().unwrap();

    println!("{:#?}", pheader);
    println!("{:#02x}", U128::from(5));
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use chrono::Utc;
    use codec::ConsensusCodec;
    use primitive_types::{H160, H256, U128, U256};
    use crate::BlockHeader;

    #[test]
    fn test_consensus_codec() {
        let block_header = BlockHeader::new(
            H256::from_str("0x0000014f092233bd0d41ab40817649d9a188ef86dc2f631a4c96e15997080499")
                .unwrap(),
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            H256::from_str("0x0c191dd909dad74ef2f96ed5dad8e9778e75b46979178cfb61f051ec06882ea8")
                .unwrap(),
            U256::from_str("0x1").unwrap(),
            H160::from_str("0x350dc631bd1dc8f21d76a636ecea2ed4482a0a97").unwrap(),
            30,
            30,
            30,
            Utc::now().timestamp_subsec_millis(),
            U128::from(5),
        );
        let a = block_header.into_proto();
        let encoded = block_header.consensus_encode();
        let block_header = BlockHeader::consensus_decode(&encoded).unwrap();
        let b = block_header.into_proto();
        assert_eq!(a.unwrap(), b.unwrap());
    }
}

