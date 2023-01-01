use core::cmp;
use std::cmp::Ordering;
use std::sync::{Arc, RwLock};
use std::u32;

use crate::account::Address42;
use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use codec::impl_codec_using_prost;
use codec::ConsensusCodec;
use codec::{Decodable, Encodable};
use crypto::dhash256;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use primitive_types::{Compact, H256, U256};
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

#[derive(
    Serialize, Deserialize, Copy, Clone, Debug, Default, Getters, Setters, MutGetters, CopyGetters,
)]
pub struct BlockHeader {
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    parent_hash: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    receipt_hash: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    tx_root: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    state_root: H256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    mix_nonce: U256,
    #[getset(get = "pub", set = "pub", get_mut = "pub")]
    coinbase: Address42,
    #[getset(set = "pub", get_mut = "pub")]
    difficulty: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    chain_id: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    level: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    time: u32,
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    nonce: u64,
}
#[allow(clippy::too_many_arguments)]
impl BlockHeader {
    pub fn new(
        parent_hash: H256,
        receipt_hash: H256,
        tx_root: H256,
        state_root: H256,
        mix_nonce: U256,
        coinbase: Address42,
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
            coinbase: Address42::from_slice(&bytes.copy_to_bytes(42))?,
            difficulty: bytes.get_u32(),
            chain_id: bytes.get_u32(),
            level: bytes.get_u32(),
            time: bytes.get_u32(),
            nonce: bytes.get_u64(),
        })
    }
}

impl prost::Message for BlockHeader {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        let mut tag = 0;
        let mut next_tag = || {
            tag += 1;
            tag
        };
        prost::encoding::bytes::encode(next_tag(), &self.parent_hash, buf);
        prost::encoding::bytes::encode(next_tag(), &self.receipt_hash, buf);
        prost::encoding::bytes::encode(next_tag(), &self.tx_root, buf);
        prost::encoding::bytes::encode(next_tag(), &self.state_root, buf);
        prost::encoding::bytes::encode(next_tag(), &self.mix_nonce, buf);
        prost::encoding::bytes::encode(next_tag(), &self.coinbase, buf);
        prost::encoding::uint32::encode(next_tag(), &self.difficulty, buf);
        prost::encoding::uint32::encode(next_tag(), &self.chain_id, buf);
        prost::encoding::uint32::encode(next_tag(), &self.level, buf);
        prost::encoding::uint32::encode(next_tag(), &self.time, buf);
        prost::encoding::uint64::encode(next_tag(), &self.nonce, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        const STRUCT_NAME: &str = "BlockHeader";
        match tag {
            1 => prost::encoding::bytes::merge(wire_type, &mut self.parent_hash, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "parent_hash");
                    error
                },
            ),
            2 => prost::encoding::bytes::merge(wire_type, &mut self.receipt_hash, buf, ctx)
                .map_err(|mut error| {
                    error.push(STRUCT_NAME, "receipt_hash");
                    error
                }),
            3 => prost::encoding::bytes::merge(wire_type, &mut self.tx_root, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "merkle_root");
                    error
                },
            ),
            4 => prost::encoding::bytes::merge(wire_type, &mut self.state_root, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "state_root");
                    error
                },
            ),
            5 => prost::encoding::bytes::merge(wire_type, &mut self.mix_nonce, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "mix_nonce");
                    error
                },
            ),
            6 => prost::encoding::bytes::merge(wire_type, &mut self.coinbase, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "coinbase");
                    error
                },
            ),
            7 => prost::encoding::uint32::merge(wire_type, &mut self.difficulty, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "difficulty");
                    error
                },
            ),
            8 => prost::encoding::uint32::merge(wire_type, &mut self.chain_id, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "chain_id");
                    error
                },
            ),
            9 => prost::encoding::uint32::merge(wire_type, &mut self.level, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "level");
                    error
                },
            ),
            10 => {
                let value = &mut self.time;
                prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "time");
                    error
                })
            }
            11 => prost::encoding::uint64::merge(wire_type, &mut self.nonce, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "nonce");
                    error
                },
            ),

            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut tag = 0;
        let mut next_tag = || {
            tag += 1;
            tag
        };
        prost::encoding::bytes::encoded_len(next_tag(), &self.parent_hash)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.receipt_hash)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.tx_root)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.state_root)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.mix_nonce)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.coinbase)
            + prost::encoding::uint32::encoded_len(next_tag(), &self.difficulty)
            + prost::encoding::uint32::encoded_len(next_tag(), &self.chain_id)
            + prost::encoding::uint32::encoded_len(next_tag(), &self.level)
            + prost::encoding::uint32::encoded_len(next_tag(), &self.time)
            + prost::encoding::uint64::encoded_len(next_tag(), &self.nonce)
    }

    fn clear(&mut self) {
        *self = BlockHeader::default()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<SignedTransaction>,
    #[serde(skip)]
    hash: Arc<RwLock<Option<H256>>>,
}

impl prost::Message for Block {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::message::encode(1, &self.header, buf);
        prost::encoding::message::encode_repeated(2, &self.transactions, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        const STRUCT_NAME: &str = "Block";
        match tag {
            1 => {
                let value = &mut self.header;
                prost::encoding::message::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "header");
                    error
                })
            }
            2 => {
                let value = &mut self.transactions;
                prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "transactions");
                        error
                    },
                )
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::message::encoded_len(1, &self.header)
            + prost::encoding::message::encoded_len_repeated(2, &self.transactions)
    }

    fn clear(&mut self) {
        *self = Default::default()
    }
}

impl Block {
    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
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
        cache(&self.hash, || self.header.hash())
    }

    pub fn header(&self) -> &BlockHeader {
        &self.header
    }
    pub fn level(&self) -> u32 {
        self.header.level
    }
    pub fn parent_hash(&self) -> &H256 {
        self.header.parent_hash()
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
