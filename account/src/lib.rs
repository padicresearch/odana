use anyhow::Result;
use codec::{Decoder, Encoder};
use ed25519_dalek::ed25519::signature::Signature;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer, Verifier};
use primitive_types::H160;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tiny_keccak::Hasher;

pub const SUDO_PUB_KEY: [u8; 32] = [
    102, 129, 71, 31, 126, 221, 234, 218, 37, 39, 104, 100, 107, 75, 80, 209, 8, 43, 33, 26, 137,
    251, 184, 15, 106, 108, 183, 54, 227, 161, 217, 70,
];

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: [u8; 20],
    pub pri_key: [u8; 32],
    pub pub_key: [u8; 32],
}

impl Encoder for Account {}
impl Decoder for Account {}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.address.eq(&other.address)
    }
}

impl Eq for Account {}

impl Hash for Account {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.address)
    }
}

impl Account {
    pub fn address_encoded(&self) -> String {
        format!("0x{}", hex::encode(self.address))
    }

    pub fn sign(&self, payload: &[u8]) -> Result<[u8; 64]> {
        let key_pair = Keypair {
            secret: SecretKey::from_bytes(&self.pri_key)?,
            public: PublicKey::from_bytes(&self.pub_key)?,
        };
        let sig = key_pair.sign(payload);
        Ok(sig.to_bytes())
    }
}

impl Into<H160> for Account {
    fn into(self) -> H160 {
        H160::from(self.address)
    }
}

pub fn create_account() -> Account {
    let mut csprng = ChaCha20Rng::from_entropy();
    let keypair = Keypair::generate(&mut csprng);
    let pri_key = keypair.secret.to_bytes();
    let pub_key = keypair.public.to_bytes();
    let mut address = [0_u8; 20];
    let mut keccak_hash = [0u8; 32];
    let mut keccak = tiny_keccak::Keccak::v256();
    keccak.update(&pub_key);
    keccak.finalize(&mut keccak_hash);
    address.copy_from_slice(&keccak_hash[keccak_hash.len() - 20..]);
    Account {
        address,
        pri_key,
        pub_key,
    }
}

pub fn verify_signature(pub_key: &[u8; 32], sig: &[u8; 64], message: &[u8]) -> Result<()> {
    let pub_key = PublicKey::from_bytes(pub_key)?;
    let sig: ed25519_dalek::Signature = Signature::from_bytes(sig)?;
    pub_key.verify(message, &sig).map_err(|e| e.into())
}

#[cfg(test)]
mod test {
    use crate::create_account;
    use ed25519_dalek::Signature;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_create_account() {
        let mut accounts = HashSet::with_capacity(1001);
        for _ in 0..1000 {
            let account = create_account();
            assert_eq!(accounts.contains(&account), false);
            accounts.insert(account);
        }
    }
}
