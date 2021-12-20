use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signer};
use tiny_keccak::Hasher;
use rand_chacha::{ChaCha12Rng, ChaCha20Rng};
use rand_chacha::rand_core::SeedableRng;
use rand_core::OsRng;
use std::hash::{Hash};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::errors::BlockChainError;
use storage::codec::{Encoder, Decoder};

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
        format!("0x{}",hex::encode(self.address))
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

#[cfg(test)]
mod test {
    use crate::account::{create_account, Account};
    use std::collections::{HashMap, HashSet};
    use ed25519_dalek::Signature;

    #[test]
    fn test_create_account() {
        let mut accounts = HashSet::with_capacity(1001);
        for _ in 0..1000 {
            let account = create_account();
            assert_eq!(accounts.contains(&account), false);
            accounts.insert(account);
        }
    }

    #[test]
    fn test_signature() {
        let account = create_account();
//Signature
        //account.sign()
    }
}