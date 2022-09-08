use crypto::sha256;

pub const HASH_LEN: usize = 32;

pub trait HashFunction {
    fn digest(&self, input: &[u8]) -> [u8; HASH_LEN];
}

#[derive(Debug, Clone)]
pub struct Sha3Keccak256;

impl HashFunction for Sha3Keccak256 {
    fn digest(&self, input: &[u8]) -> [u8; HASH_LEN] {
        sha256(input).into()
    }
}
