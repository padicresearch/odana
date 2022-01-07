use anyhow::Result;
use codec::impl_codec;
use codec::{Decoder, Encoder};
use serde::{Deserialize, Serialize};
use primitive_types::H160;
use std::hash::Hash;
use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signer};

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    pub free_balance: u128,
    pub reserve_balance: u128,
    pub nonce: u64,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            free_balance: 0,
            reserve_balance: 0,
            nonce: 0,
        }
    }
}

impl_codec!(AccountState);



#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: H160,
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
        state.write(&self.address.as_bytes())
    }
}

impl Account {
    pub fn address_encoded(&self) -> String {
        format!("{}", self.address)
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
        self.address
    }
}
