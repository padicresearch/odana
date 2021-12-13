use anyhow::Result;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

pub trait Encoder : Sized + Serialize + DeserializeOwned {
    fn encode(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e|e.into())
    }
}

pub trait Decoder : Sized + Serialize + DeserializeOwned {
    fn decode(buf : &[u8]) -> Result<Self> {
        bincode::deserialize(buf).map_err(|e|e.into())
    }
}

pub trait Codec : Encoder + Decoder {}

impl<T> Codec for T where T : Encoder + Decoder {}


type Hash = [u8;32];

impl Encoder for Hash {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }
}

impl Decoder for Hash {
    fn decode(buf: &[u8]) -> Result<Self> {
        let mut buff = [0;32];
        buff.copy_from_slice(buf);
        Ok(buff)
    }
}