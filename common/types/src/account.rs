use anyhow::{anyhow, bail, Result};
use serde::ser::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;

use bech32::{ToBase32, Variant};
use serde::{Deserialize, Serialize};

use crate::network::{Network, ALPHA_HRP, MAINNET_HRP, TESTNET_HRP};
use codec::{Decodable, Encodable};
use crypto::ecdsa::{PublicKey, SecretKey, Signature};
use crypto::keccak256;
use primitive_types::{H160, H256};

pub const ADDRESS_LEN: usize = 44;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, prost::Message)]
pub struct AccountState {
    #[prost(uint64, tag = "1")]
    pub free_balance: u64,
    #[prost(uint64, tag = "2")]
    pub reserve_balance: u64,
    #[prost(uint64, tag = "3")]
    pub nonce: u64,
}

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            free_balance: 0u64,
            reserve_balance: 0u64,
            nonce: 1u64,
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

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct Address(pub [u8; 44]);

impl Default for Address {
    fn default() -> Self {
        Self([0; 44])
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        f.write_str(&s[..6])?;
        f.write_str("...")?;
        f.write_str(&s[36..])?;
        Ok(())
    }
}

impl From<[u8; ADDRESS_LEN]> for Address {
    fn from(slice: [u8; ADDRESS_LEN]) -> Self {
        Address(slice)
    }
}

impl prost::encoding::BytesAdapter for Address {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace_with<B>(&mut self, mut buf: B)
        where
            B: prost::bytes::Buf,
    {
        let buf = buf.copy_to_bytes(buf.remaining());
        match Address::from_slice(buf.as_ref()) {
            Ok(addr) => {
                *self = addr;
            }
            Err(_) => {}
        }
    }

    fn append_to<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
    {
        buf.put_slice(self.as_bytes())
    }
}

impl Encodable for Address {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.0.to_vec())
    }
}

impl Decodable for Address {
    fn decode(buf: &[u8]) -> Result<Self> {
        Address::from_slice(buf)
    }
}

impl Address {
    pub fn is_mainnet(&self) -> bool {
        MAINNET_HRP.eq(&self.hrp())
    }
    pub fn is_testnet(&self) -> bool {
        TESTNET_HRP.eq(&self.hrp())
    }
    pub fn is_alphanet(&self) -> bool {
        ALPHA_HRP.eq(&self.hrp())
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        let mut bytes = [0; ADDRESS_LEN];
        if slice.len() != bytes.len() {
            bail!("decode error")
        }
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn hrp(&self) -> String {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp, _, _)) => hrp,
            Err(_) => String::new(),
        }
    }
    pub fn is_valid(&self) -> bool {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp, _, _)) => ALPHA_HRP.eq(&hrp) || TESTNET_HRP.eq(&hrp) || MAINNET_HRP.eq(&hrp),
            Err(_) => false,
        }
    }

    pub fn network(&self) -> Option<Network> {
        match self.hrp().as_str() {
            MAINNET_HRP => Some(Network::Mainnet),
            TESTNET_HRP => Some(Network::Testnet),
            ALPHA_HRP => Some(Network::Alphanet),
            _ => None,
        }
    }

    pub fn to_address20(&self) -> Option<H160> {
        match bech32::decode(&String::from_utf8_lossy(&self.0))
            .and_then(|(_, address_32, _)| bech32::convert_bits(&address_32, 5, 8, false))
        {
            Ok(address) => Some(H160::from_slice(&address)),
            Err(_) => None,
        }
    }
}

impl FromStr for Address {
    type Err = bech32::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if !input.len() == ADDRESS_LEN {
            return Err(Self::Err::InvalidLength);
        }
        let _ = bech32::decode(input)?;
        let mut bytes = [0; ADDRESS_LEN];
        bytes.copy_from_slice(input.as_bytes());
        Ok(Address(bytes))
    }
}

struct Address42Visitor;

impl<'b> serde::de::Visitor<'b> for Address42Visitor {
    type Value = Address;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string with len {}", ADDRESS_LEN)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if !v.len() == ADDRESS_LEN {
            return Err(E::invalid_length(v.len(), &self));
        }
        let _ = bech32::decode(v).map_err(|e| E::custom(e))?;
        let mut bytes = [0; ADDRESS_LEN];
        bytes.copy_from_slice(v.as_bytes());
        Ok(Address(bytes))
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }
}

impl ::serde::Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
    {
        serializer.serialize_str(
            &String::from_utf8(self.0.to_vec()).map_err(|e| S::Error::custom(&e.to_string()))?,
        )
    }
}

impl<'de> ::serde::Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(Address42Visitor)
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
    use crate::account::{Address, get_address_from_app_id};
    use serde::{Deserialize, Serialize};
    use bech32::{ToBase32, Variant};
    use crypto::keccak256;
    use crate::network::Network;

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
