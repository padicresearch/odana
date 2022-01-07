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
use crypto::{Ripe160, SHA256};
use types::account::Account;
use types::PubKey;

pub const GOVERNANCE_ACCOUNTID: [u8; 32] = [
    102, 129, 71, 31, 126, 221, 234, 218, 37, 39, 104, 100, 107, 75, 80, 209, 8, 43, 33, 26, 137,
    251, 184, 15, 106, 108, 183, 54, 227, 161, 217, 70,
];


pub fn create_account() -> Account {
    let mut csprng = ChaCha20Rng::from_entropy();
    let keypair = Keypair::generate(&mut csprng);
    let pri_key = keypair.secret.to_bytes();
    let pub_key = keypair.public.to_bytes();
    let mut address = Ripe160::digest(SHA256::digest(&pub_key).as_bytes());
    Account {
        address,
        pri_key,
        pub_key,
    }
}

pub fn get_address_from_pub_key(pub_key: &PubKey) -> H160 {
    Ripe160::digest(SHA256::digest(&pub_key).as_bytes())
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
