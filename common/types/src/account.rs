use std::hash::Hash;
use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use codec::impl_codec;
use codec::{Decoder, Encoder};
use crypto::ecdsa::{PublicKey, SecretKey, Signature};
use crypto::{RIPEMD160, SHA256};
use primitive_types::{H160, H256};

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    #[serde(with = "crate::uint_hex_codec")]
    pub free_balance: u128,
    #[serde(with = "crate::uint_hex_codec")]
    pub reserve_balance: u128,
    #[serde(with = "crate::uint_hex_codec")]
    pub nonce: u64,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            free_balance: 0,
            reserve_balance: 0,
            nonce: 1,
        }
    }
}

impl_codec!(AccountState);

impl AccountState {
    pub fn into_proto(self) -> Result<proto::AccountState> {
        let json_rep = serde_json::to_vec(&self)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: H160,
    pub secret: H256,
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
        let secrete = SecretKey::from_bytes(self.secret.as_fixed_bytes())?;
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

pub fn get_address_from_secret_key(key: H256) -> Result<H160> {
    let secret = SecretKey::from_bytes(key.as_bytes())?;
    Ok(get_address_from_pub_key(secret.public()))
}
