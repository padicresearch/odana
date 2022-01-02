use sha2::{Digest, Sha256};

pub const HASH_LEN: usize = 32;
pub const BLOCK_HASH_LEN: usize = 32;

pub trait HashFunction {
    fn digest(&self, input: &[u8]) -> [u8; HASH_LEN];
}

#[derive(Debug, Clone)]
pub struct SHA256;

impl HashFunction for SHA256 {
    fn digest(&self, input: &[u8]) -> [u8; HASH_LEN] {
        let out = Sha256::digest(input);
        out.into()
    }
}
