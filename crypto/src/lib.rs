#![no_std]
#![feature(error_in_core)]
extern crate alloc;
extern crate core;

pub use digest::Digest;
use primitive_types::{Compact, H160, H256, H448, U192, U256};
use ripemd::Ripemd160;
use sha2::Sha256;

pub mod ecdsa;
mod error;

pub struct SHA256;

impl SHA256 {
    pub fn digest<B: AsRef<[u8]>>(bytes: B) -> H256 {
        let mut sha = Sha256::default();
        sha.update(bytes.as_ref());
        H256::from_slice(sha.finalize().as_ref())
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

#[inline]
pub fn ripemd160<B: AsRef<[u8]>>(bytes: B) -> H160 {
    let mut hasher = Ripemd160::default();
    hasher.update(bytes);
    H160::from_slice(hasher.finalize().as_ref())
}
#[inline]
pub fn dhash160<B: AsRef<[u8]>>(bytes: B) -> H160 {
    ripemd160(sha256(bytes))
}

#[inline]
pub fn dhash256<B: AsRef<[u8]>>(bytes: B) -> H256 {
    sha256(sha256(bytes))
}
#[inline]
pub fn sha256<B: AsRef<[u8]>>(bytes: B) -> H256 {
    let mut hasher = Sha256::default();
    hasher.update(bytes);
    H256::from_slice(hasher.finalize().as_ref())
}

#[inline]
pub fn keccak256<B: AsRef<[u8]>>(bytes: B) -> H256 {
    let mut hasher = sha3::Keccak256::default();
    hasher.update(bytes.as_ref());
    let out = hasher.finalize();
    H256::from_slice(out.as_ref())
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
    let m = if frac.abs() < f64::EPSILON {
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
