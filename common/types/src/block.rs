use core::cmp;
use std::cmp::Ordering;
use std::sync::Arc;
use std::u32;

use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use codec::impl_codec_using_prost;
use codec::ConsensusCodec;
use codec::{Decodable, Encodable};
use crypto::dhash256;

use primitive_types::address::Address;
use primitive_types::{Compact, ADDRESS_LEN, H256, U256};
use serde::{Deserialize, Serialize};

use crate::tx::SignedTransaction;

use super::*;

const HEADER_SIZE: usize = 212;

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug, Serialize, Deserialize)]
pub struct BlockPrimaryKey(pub u32, pub H256);

impl Encodable for BlockPrimaryKey {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::with_capacity(36);
        encoded.extend_from_slice(&self.0.to_be_bytes());
        encoded.extend_from_slice(self.1.as_bytes());

        Ok(encoded)
    }
}

impl Decodable for BlockPrimaryKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut level: [u8; 4] = [0; 4];
        level.copy_from_slice(&buf[..4]);
        let level = u32::from_be_bytes(level);
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&buf[4..]);
        Ok(Self(level, hash.into()))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, prost::Message)]
pub struct BlockHeader {
    #[prost(required, message, tag = "1")]
    pub parent_hash: H256,
    #[prost(required, message, tag = "2")]
    pub receipt_hash: H256,
    #[prost(required, message, tag = "3")]
    pub tx_root: H256,
    #[prost(required, message, tag = "4")]
    pub state_root: H256,
    #[prost(required, message, tag = "5")]
    pub mix_nonce: U256,
    #[prost(required, message, tag = "6")]
    pub coinbase: Address,
    #[prost(uint32, tag = "7")]
    pub difficulty: u32,
    #[prost(uint32, tag = "8")]
    pub chain_id: u32,
    #[prost(uint32, tag = "9")]
    pub level: u32,
    #[prost(uint32, tag = "10")]
    pub time: u32,
    #[prost(uint64, tag = "11")]
    pub nonce: u64,
}
#[allow(clippy::too_many_arguments)]
impl BlockHeader {
    pub fn new(
        parent_hash: H256,
        receipt_hash: H256,
        tx_root: H256,
        state_root: H256,
        mix_nonce: U256,
        coinbase: Address,
        difficulty: u32,
        chain_id: u32,
        level: u32,
        time: u32,
        nonce: u64,
    ) -> Self {
        Self {
            parent_hash,
            receipt_hash,
            tx_root,
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
        dhash256(self.consensus_encode())
    }

    pub fn difficulty(&self) -> Compact {
        Compact::from(self.difficulty)
    }
}

impl ConsensusCodec for BlockHeader {
    fn consensus_encode(self) -> Vec<u8> {
        let mut encoded = BytesMut::with_capacity(HEADER_SIZE);
        encoded.extend(self.parent_hash.as_bytes());
        encoded.extend(self.receipt_hash.as_bytes());
        encoded.extend(self.tx_root.as_bytes());
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
            receipt_hash: H256::from_slice(&bytes.copy_to_bytes(32)),
            tx_root: H256::from_slice(&bytes.copy_to_bytes(32)),
            state_root: H256::from_slice(&bytes.copy_to_bytes(32)),
            mix_nonce: U256::from_big_endian(&bytes.copy_to_bytes(32)),
            coinbase: Address::from_slice_checked(&bytes.copy_to_bytes(ADDRESS_LEN))
                .map_err(|_| anyhow::anyhow!("error decoding coinbase address"))?,
            difficulty: bytes.get_u32(),
            chain_id: bytes.get_u32(),
            level: bytes.get_u32(),
            time: bytes.get_u32(),
            nonce: bytes.get_u64(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<SignedTransaction>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<H256>>>,
}

impl prost::Message for Block {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        ::prost::encoding::message::encode(1u32, &self.header, buf);
        for msg in &self.transactions {
            prost::encoding::message::encode(2u32, msg, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> core::result::Result<(), DecodeError>
    where
        B: Buf,
    {
        const STRUCT_NAME: &'static str = stringify!(Block);
        match tag {
            1u32 => {
                let value = &mut self.header;
                prost::encoding::message::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(header));
                    error
                })
            }
            2u32 => {
                let value = &mut self.transactions;
                prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, stringify!(transactions));
                        error
                    },
                )
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        prost::encoding::message::encoded_len(1u32, &self.header)
            + prost::encoding::message::encoded_len_repeated(2u32, &self.transactions)
    }
    fn clear(&mut self) {
        self.header.clear();
        self.transactions.clear();
    }
}

impl Block {
    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.transactions
    }
}

impl_codec_using_prost!(BlockHeader);
impl_codec_using_prost!(Block);

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
        cache(&self.hash, || Ok(self.header.hash()))
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }
    pub fn level(&self) -> u32 {
        self.header.level
    }
    pub fn parent_hash(&self) -> &H256 {
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
