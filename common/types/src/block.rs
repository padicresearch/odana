use core::cmp;
use std::cmp::Ordering;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use codec::ConsensusCodec;
use codec::{Decodable, Encodable};
use crypto::{dhash256};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use hex::{FromHex, ToHex};
use primitive_types::{Compact, H256, U128, U256};
use serde::{Deserialize, Serialize};

use crate::tx::SignedTransaction;

use super::*;

const HEADER_SIZE: usize = 180;

#[derive(
    Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug, Serialize, Deserialize, Encode, Decode,
)]
pub struct BlockPrimaryKey(pub Hash, pub i32);

impl Encodable for BlockPrimaryKey {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut encoded = Vec::with_capacity(36);
        encoded.extend(self.1.to_be_bytes());
        encoded.extend(self.0.iter());

        Ok(encoded)
    }
}

impl Decodable for BlockPrimaryKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut level: [u8; 4] = [0; 4];
        level.copy_from_slice(&buf[..4]);
        let level = i32::from_be_bytes(level);
        let mut hash: [u8; 32] = [0; 32];
        hash.copy_from_slice(&buf[4..]);
        Ok(Self(hash, level))
    }
}

#[derive(
    Serialize, Deserialize, Copy, Clone, Debug, Default, Getters, Setters, MutGetters, CopyGetters,
)]
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
    chain_id: u16,
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
        chain_id: u16,
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
            chain_id: bytes.get_u16(),
            level: bytes.get_i32(),
            time: bytes.get_u32(),
            nonce: U128::from(bytes.get_u128()),
        })
    }
}

impl prost::Message for BlockHeader {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::string::encode(1, &self.parent_hash.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(2, &self.merkle_root.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(3, &self.state_root.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(4, &self.mix_nonce.encode_hex(), buf);
        prost::encoding::string::encode(5, &self.coinbase.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::uint32::encode(6, &self.difficulty, buf);
        prost::encoding::uint32::encode(7, &(self.chain_id as u32), buf);
        prost::encoding::int32::encode(8, &self.level, buf);
        prost::encoding::uint32::encode(9, &self.time, buf);
        prost::encoding::string::encode(10, &self.nonce.encode_hex(), buf);
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
        const STRUCT_NAME: &'static str = "BlockHeader";
        match tag {
            1 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "parent_hash");
                        error
                    },
                )?;
                self.parent_hash = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "parent_hash");
                    error
                })?;
                Ok(())
            }
            2 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "merkle_root");
                        error
                    },
                )?;
                self.merkle_root = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "merkle_root");
                    error
                })?;
                Ok(())
            }
            3 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "state_root");
                        error
                    },
                )?;
                self.state_root = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "state_root");
                    error
                })?;
                Ok(())
            }
            4 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "mix_nonce");
                        error
                    },
                )?;
                self.mix_nonce = U256::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "mix_nonce");
                    error
                })?;
                Ok(())
            }
            5 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "coinbase");
                        error
                    },
                )?;
                self.coinbase = H160::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "coinbase");
                    error
                })?;
                Ok(())
            }
            6 => {
                let value = &mut self.difficulty;
                prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "difficulty");
                        error
                    },
                )
            }
            7 => {
                let mut value : u32 = 0;
                prost::encoding::uint32::merge(wire_type, &mut value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "difficulty");
                        error
                    },
                )?;
                self.chain_id = value as u16;
                Ok(())
            }
            8 => {
                let value = &mut self.level;
                prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "level");
                        error
                    },
                )
            }
            9 => {
                let value = &mut self.time;
                prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "time");
                        error
                    },
                )
            }

            10 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "nonce");
                        error
                    },
                )?;
                self.nonce = U128::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "nonce");
                    error
                })?;
                Ok(())
            }

            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::string::encoded_len(1, &self.parent_hash.as_fixed_bytes().encode_hex())
            + prost::encoding::string::encoded_len(
                2,
                &self.merkle_root.as_fixed_bytes().encode_hex(),
            )
            + prost::encoding::string::encoded_len(
                3,
                &self.state_root.as_fixed_bytes().encode_hex(),
            )
            + prost::encoding::string::encoded_len(4, &self.mix_nonce.encode_hex())
            + prost::encoding::string::encoded_len(5, &self.coinbase.as_fixed_bytes().encode_hex())
            + prost::encoding::uint32::encoded_len(6, &self.difficulty)
            + prost::encoding::uint32::encoded_len(7, &(self.chain_id as u32))
            + prost::encoding::int32::encoded_len(8, &self.level)
            + prost::encoding::uint32::encoded_len(9, &self.time)
            + prost::encoding::string::encoded_len(10, &self.nonce.encode_hex())
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
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut, Self: Sized {
        todo!()
    }

    fn merge_field<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B, ctx: DecodeContext) -> std::result::Result<(), DecodeError> where B: Buf, Self: Sized {
        todo!()
    }

    fn encoded_len(&self) -> usize {
        todo!()
    }

    fn clear(&mut self) {
        todo!()
    }
}

impl Block {
    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
    }
}

impl Encodable for BlockHeader {
    fn encode(&self) -> Result<Vec<u8>> {
        todo!()
    }
}

impl Decodable for BlockHeader {
    fn decode(buf: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl Encodable for Block {
    fn encode(&self) -> Result<Vec<u8>> {
        todo!()
    }
}

impl Decodable for Block {
    fn decode(buf: &[u8]) -> Result<Self> {
        todo!()
    }
}

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

    let pheader = block_header.encode_to_vec();
    println!("{}", hex::encode(pheader, false))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use chrono::Utc;
    use prost::Message;

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
        let a = block_header.encode_to_vec();
        let encoded = block_header.consensus_encode();
        let block_header = BlockHeader::consensus_decode(&encoded).unwrap();
        let b = block_header.encode_to_vec();
        assert_eq!(a, b);
    }
}
