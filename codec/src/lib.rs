#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use anyhow::Result;
use bincode::config::{BigEndian, Fixint, NoLimit, SkipFixedArrayLength};
use primitive_types::address::Address;
use primitive_types::{H160, H256};

pub trait ConsensusCodec: Sized {
    fn consensus_encode(self) -> Vec<u8>;
    fn consensus_decode(buf: &[u8]) -> Result<Self>;
}

pub trait Encodable: Sized {
    fn encode(&self) -> Result<Vec<u8>>;
}

pub trait Decodable: Sized {
    fn decode(buf: &[u8]) -> Result<Self>;
}

pub trait Codec: Encodable + Decodable {}

impl<T> Codec for T where T: Encodable + Decodable {}

type Hash = [u8; 32];

impl Encodable for Hash {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }
}

impl Decodable for Hash {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut buff = [0; 32];
        buff.copy_from_slice(buf);
        Ok(buff)
    }
}

impl Encodable for Vec<u8> {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.clone())
    }
}

impl Decodable for Vec<u8> {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(buf.to_vec())
    }
}

macro_rules! impl_codec_primitives {
    ($name : ident) => {
        impl Encodable for $name {
            fn encode(&self) -> Result<Vec<u8>> {
                Ok(self.to_be_bytes().to_vec())
            }
        }

        impl Decodable for $name {
            fn decode(buf: &[u8]) -> Result<$name> {
                Ok($name::from_be_bytes(buf.try_into()?))
            }
        }
    };
}

#[macro_export]
macro_rules! impl_codec_using_prost {
    ($name : ident) => {
        impl Encodable for $name {
            fn encode(&self) -> Result<Vec<u8>> {
                Ok(prost::Message::encode_to_vec(self))
            }
        }

        impl Decodable for $name {
            fn decode(buf: &[u8]) -> Result<$name> {
                prost::Message::decode(buf).map_err(|e| e.into())
            }
        }
    };
}

impl_codec_primitives!(u8);
impl_codec_primitives!(u16);
impl_codec_primitives!(u32);
impl_codec_primitives!(u64);
impl_codec_primitives!(u128);

impl Encodable for String {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.as_bytes().to_vec())
    }
}

impl Decodable for String {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(String::from_utf8_lossy(buf).to_string())
    }
}

impl Encodable for H160 {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.as_bytes().to_vec())
    }
}

impl Decodable for H160 {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(H160::from_slice(buf))
    }
}

impl Encodable for H256 {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.as_bytes().to_vec())
    }
}

impl Decodable for H256 {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(H256::from_slice(buf))
    }
}

impl Encodable for Address {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.0.to_vec())
    }
}

impl Decodable for Address {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(Address::from_slice(buf))
    }
}

pub fn config() -> bincode::config::Configuration<BigEndian, Fixint, SkipFixedArrayLength, NoLimit>
{
    bincode::config::standard()
        .with_big_endian()
        .with_fixed_int_encoding()
        .skip_fixed_array_length()
}
