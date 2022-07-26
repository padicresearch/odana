use primitive_types::{Compact, H160, H256, H448, U192, U256};
use ripemd::{Digest, Ripemd160};
use sha2::digest::{FixedOutput, FixedOutputDirty};
use sha2::Digest as ShaDigest;
use std::io::Write;
use tiny_keccak::Hasher;

pub mod ecdsa;
mod error;

pub struct SHA256;

impl SHA256 {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> H256 {
        let mut out = H256::zero();
        let mut sha = sha2::Sha256::new();
        sha.update(bytes.as_ref());
        out = H256::from_slice(sha.finalize().as_slice());
        out
    }

    pub fn concat_digest<'a, I: IntoIterator<Item=&'a [u8]>>(items: I) -> H256 {
        let mut out = H256::zero();
        let mut sha = sha2::Sha256::new();
        for i in items {
            sha.update(i);
        }
        out = H256::from_slice(sha.finalize().as_slice());
        out
    }
}

pub struct RIPEMD160;

impl RIPEMD160 {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> H160 {
        let mut hasher = Ripemd160::new();
        hasher.update(bytes);
        let out: [u8; 20] = <[u8; 20]>::from(hasher.finalize());
        H160::from(out)
    }
}

#[cfg(test)]
mod test {
    use crate::{RIPEMD160, SHA256};

    #[test]
    fn test_hashes() {
        let hello = SHA256::digest(b"hello");
        println!("{:?}", hello.as_fixed_bytes());
        println!("{:?}", RIPEMD160::digest(hello.as_bytes()));
    }
}

pub fn is_valid_proof_of_work(max_work_bits: Compact, bits: Compact, hash: &H256) -> bool {
    let maximum = match max_work_bits.to_u256() {
        Ok(max) => max,
        _err => return false,
    };

    let target = match bits.to_u256() {
        Ok(target) => target,
        _err => return false,
    };
    let value = U256::from(hash.as_fixed_bytes());

    target <= maximum && value <= target
}

/// Returns true if hash is lower or equal than target represented by compact bits
pub fn is_valid_proof_of_work_hash(target: U256, hash: &H256) -> bool {
    let value = U256::from(hash.as_fixed_bytes());
    value <= target
}

pub fn generate_pow_from_pub_key(pub_key: H256, target: U256) -> (U192, H448) {
    let mut nonce = U192::zero();
    let mut pow_stamp = [0_u8; 56];
    pow_stamp[24..].copy_from_slice(pub_key.as_bytes());
    loop {
        pow_stamp[..24].copy_from_slice(&nonce.to_le_bytes());
        let h = SHA256::digest(pow_stamp);
        if is_valid_proof_of_work_hash(target, &h) {
            return (nonce, H448::from(pow_stamp));
        }
        nonce += U192::one();
    }
}

pub fn make_target(target: f64) -> U256 {
    assert!((0.0..256.0).contains(&target));
    let (frac, shift) = (target.fract(), target.floor() as u64);
    let m = if frac.abs() < std::f64::EPSILON {
        (1 << 54) - 1
    } else {
        2.0f64.powf(54.0 - frac) as u64
    };
    let m = U256::from(m);
    if shift < 202 {
        (m << (202 - shift)) | ((U256::from(1u64) << (202 - shift)) - U256::from(1u64))
    } else {
        m >> (shift - 202)
    }
}

