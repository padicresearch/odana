use anyhow::Result;

pub trait Encoder : Sized {
    fn encode(&self) -> Result<Vec<u8>>;
}

pub trait Decoder: Sized {
    fn decode(buf : &[u8]) -> Result<Self>;
}

pub trait Codec : Encoder + Decoder {}

impl<T> Codec for T where T : Encoder + Decoder {}
