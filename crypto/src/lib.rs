use primitive_types::{H256, H160};
use tiny_keccak::Hasher;
use ripemd::{Ripemd160, Digest};

pub struct SHA256;

impl SHA256 {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> H256 {
        let mut out = H256::zero();
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(bytes.as_ref());
        sha3.finalize(out.as_bytes_mut());
        out
    }
}

pub struct Ripe160;

impl Ripe160 {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> H160 {
        let mut hasher = Ripemd160::new();
        hasher.update(bytes);
        let out: [u8; 20] = <[u8; 20]>::from(hasher.finalize());
        H160::from(out)
    }
}


#[cfg(test)]
mod test {
    use crate::{SHA256, Ripe160};

    #[test]
    fn test_hashes() {
        let hello = SHA256::digest(b"hello");
        println!("{:?}", hello.as_fixed_bytes());
        println!("{:?}", Ripe160::digest(hello.as_bytes()));
    }
}