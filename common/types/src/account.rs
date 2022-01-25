use std::hash::Hash;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use codec::{Decoder, Encoder};
use codec::impl_codec;
use crypto::{RIPEMD160, SHA256};
use crypto::ecdsa::{PublicKey, SecretKey, Signature};
use primitive_types::H160;

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
}

impl_codec!(Account);

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
    pub fn sign(&self, payload: &[u8]) -> Result<Signature> {
        let secrete = SecretKey::from_bytes(&self.pri_key)?;
        secrete.sign(payload).map_err(|e| e.into())
    }
}

impl Into<H160> for Account {
    fn into(self) -> H160 {
        self.address
    }
}

pub fn get_address_from_pub_key(pub_key: PublicKey) -> H160 {
    let mut address = RIPEMD160::digest(SHA256::digest(&pub_key.to_bytes()).as_bytes());
    address
}
