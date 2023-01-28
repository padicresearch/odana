use anyhow::{anyhow, bail, Result};
use serde::ser::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;

use bech32::{ToBase32, Variant};
use serde::{Deserialize, Serialize};

use crate::network::{Network, ALPHA_HRP, MAINNET_HRP, TESTNET_HRP};
use crate::Addressing;
use codec::{Decodable, Encodable};
use crypto::ecdsa::{PublicKey, SecretKey, Signature};
use crypto::keccak256;
use primitive_types::{Address, ADDRESS_LEN, H160, H256};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, prost::Message)]
pub struct AccountState {
    #[prost(uint64, tag = "1")]
    pub free_balance: u64,
    #[prost(uint64, tag = "2")]
    pub reserve_balance: u64,
    #[prost(uint64, tag = "3")]
    pub nonce: u64,
    #[prost(optional, bytes, tag = "4")]
    pub root_hash: Option<Vec<u8>>,
    #[prost(optional, bytes, tag = "5")]
    pub code_hash: Option<Vec<u8>>,
}

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            free_balance: 0u64,
            reserve_balance: 0u64,
            nonce: 1u64,
            root_hash: None,
            code_hash: None,
        }
    }
}

impl Encodable for AccountState {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(prost::Message::encode_to_vec(self))
    }
}

impl Decodable for AccountState {
    fn decode(buf: &[u8]) -> Result<Self> {
        <AccountState as prost::Message>::decode(buf).map_err(|e| anyhow!(e))
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: Address,
    pub secret: H256,
}

impl Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.address, f)
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.address.eq(&other.address)
    }
}

impl Eq for Account {}

impl Hash for Account {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.address.0)
    }
}

impl Account {
    pub fn sign(&self, payload: &[u8]) -> Result<Signature> {
        let secrete = SecretKey::from_bytes(self.secret.as_fixed_bytes())?;
        secrete.sign(payload).map_err(|e| e.into())
    }
}

impl From<Account> for H160 {
    fn from(account: Account) -> Self {
        account.address.to_address20().unwrap()
    }
}

impl Addressing for Address {
    fn is_mainnet(&self) -> bool {
        MAINNET_HRP.eq(&self.hrp())
    }
    fn is_testnet(&self) -> bool {
        TESTNET_HRP.eq(&self.hrp())
    }
    fn is_alphanet(&self) -> bool {
        ALPHA_HRP.eq(&self.hrp())
    }
    fn is_valid(&self) -> bool {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp, _, _)) => ALPHA_HRP.eq(&hrp) || TESTNET_HRP.eq(&hrp) || MAINNET_HRP.eq(&hrp),
            Err(_) => false,
        }
    }
    fn network(&self) -> Option<Network> {
        match self.hrp().as_str() {
            MAINNET_HRP => Some(Network::Mainnet),
            TESTNET_HRP => Some(Network::Testnet),
            ALPHA_HRP => Some(Network::Alphanet),
            _ => None,
        }
    }
}

pub fn get_address_from_pub_key(pub_key: PublicKey, network: Network) -> Address {
    let key = pub_key.hash();
    let checksum = &key[12..];
    let address: String = bech32::encode(network.hrp(), checksum.to_base32(), Variant::Bech32m)
        .expect("error creating account id");
    let mut raw_address = [0; ADDRESS_LEN];
    raw_address.copy_from_slice(address.as_bytes());
    Address(raw_address)
}

pub fn get_address_from_secret_key(sk: H256, network: Network) -> Result<Address> {
    let sk = SecretKey::from_bytes(sk.as_bytes())?;
    let pk = sk.public();
    let key = pk.hash();
    let checksum = &key[12..];
    let address: String = bech32::encode(network.hrp(), checksum.to_base32(), Variant::Bech32m)
        .expect("error creating account id");
    let mut raw_address = [0; ADDRESS_LEN];
    raw_address.copy_from_slice(address.as_bytes());
    Ok(Address(raw_address))
}

pub fn get_address_from_app_id(app_id: &[u8; 4], network: Network) -> Result<Address> {
    let key = keccak256(app_id);
    let checksum = &key[12..];
    let address: String = bech32::encode(network.hrp(), checksum.to_base32(), Variant::Bech32m)
        .expect("error creating account id");
    let mut raw_address = [0; ADDRESS_LEN];
    raw_address.copy_from_slice(&address.as_bytes()[0..ADDRESS_LEN]);
    Ok(Address(raw_address))
}

pub fn get_eth_address_from_pub_key(pub_key: PublicKey) -> H160 {
    let pubkey_bytes = pub_key.to_bytes();
    let key = keccak256(&pubkey_bytes[1..]).to_fixed_bytes();
    let checksum = &key[12..];
    H160::from_slice(checksum)
}

#[cfg(test)]
mod tests {
    use crate::account::{get_address_from_app_id, Address};
    use crate::network::Network;
    use bech32::{ToBase32, Variant};
    use crypto::keccak256;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct CAccount {
        account_id: Address,
        balance: i32,
    }

    #[test]
    fn test_address_derv() {
        let address = get_address_from_app_id(b"nick", Network::Mainnet).unwrap();
        println!("{}", address);
    }
}
