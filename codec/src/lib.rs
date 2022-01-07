use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use primitive_types::H160;

pub trait Encoder: Sized + Serialize + DeserializeOwned {
    fn encode(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| e.into())
    }
}

pub trait Decoder: Sized + Serialize + DeserializeOwned {
    fn decode(buf: &[u8]) -> Result<Self> {
        bincode::deserialize(buf).map_err(|e| e.into())
    }
}

pub trait Codec: Encoder + Decoder {}

impl<T> Codec for T where T: Encoder + Decoder {}

type Hash = [u8; 32];

impl Encoder for Hash {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }
}

impl Decoder for Hash {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut buff = [0; 32];
        buff.copy_from_slice(buf);
        Ok(buff)
    }
}

#[macro_export]
macro_rules! impl_codec {
    ($type : ty) => {
        impl Encoder for $type {}
        impl Decoder for $type {}
    };
}
macro_rules! impl_codec_primitives {
    ($type : ty => $path : path) => {
        impl Encoder for $type {
            fn encode(&self) -> Result<Vec<u8>> {
                Ok(self.to_be_bytes().to_vec())
            }
        }

        impl Decoder for $type {
            fn decode(buf: &[u8]) -> Result<$type> {
                Ok($path(buf.try_into()?))
            }
        }
    };
}

impl_codec_primitives!(u8 => u8::from_be_bytes);
impl_codec_primitives!(u16 => u16::from_be_bytes);
impl_codec_primitives!(u32 => u32::from_be_bytes);
impl_codec_primitives!(u64 => u64::from_be_bytes);
impl_codec_primitives!(u128 => u128::from_be_bytes);


impl Encoder for H160 {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.as_bytes().to_vec())
    }
}

impl Decoder for H160 {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(H160::from_slice(buf))
    }
}