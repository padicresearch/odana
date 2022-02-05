use ripemd::{Digest, Ripemd160};
use tiny_keccak::Hasher;

use primitive_types::{Compact, H160, H256, H448, U192, U256};

pub mod ecdsa;
mod error;

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
pub fn is_valid_proof_of_work_hash(bits: Compact, hash: &H256) -> bool {
    let target = match bits.to_u256() {
        Ok(target) => target,
        _err => return false,
    };

    let value = U256::from(hash.as_fixed_bytes());
    value <= target
}

pub fn generate_pow_from_pub_key(pub_key: H256, target: Compact) -> (U192, H448) {
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
