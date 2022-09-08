use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::DecodeError;
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, prost::Message)]
pub struct AccountState {
    #[prost(uint64, tag = "1")]
    pub free_balance: u64,
    #[prost(uint64, tag = "2")]
    pub reserve_balance: u64,
    #[prost(uint64, tag = "3")]
    pub nonce: u64,
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
    pub address: Address42,
    pub secret: H256,
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

#[derive(Copy, Clone, PartialOrd, Ord)]
pub struct Address42(pub [u8; 42]);

impl Default for Address42 {
    fn default() -> Self {
        Self { 0: [0; 42] }
    }
}

impl Debug for Address42 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl Display for Address42 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        f.write_str(&s[..6])?;
        f.write_str("...")?;
        f.write_str(&s[36..])?;
        Ok(())
    }
}

impl prost::encoding::BytesAdapter for Address42 {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace_with<B>(&mut self, mut buf: B)
    where
        B: prost::bytes::Buf,
    {
        let buf = buf.copy_to_bytes(buf.remaining());
        *self = Address42::from_slice(buf.as_ref());
    }

    fn append_to<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
    {
        buf.put_slice(self.as_bytes())
    }
}

impl Encodable for Address42 {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.0.to_vec())
    }
}

impl Decodable for Address42 {
    fn decode(buf: &[u8]) -> Result<Self> {
        Ok(Address42::from_slice(buf))
    }
}

impl Address42 {
    pub fn is_mainnet(&self) -> bool {
        MAINNET_HRP.eq(&self.hrp())
    }
    pub fn is_testnet(&self) -> bool {
        TESTNET_HRP.eq(&self.hrp())
    }
    pub fn is_alphanet(&self) -> bool {
        ALPHA_HRP.eq(&self.hrp())
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        let mut bytes = [0; 42];
        bytes.copy_from_slice(slice);
        Self(bytes)
    }


    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
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

impl FromStr for Address42 {
    type Err = bech32::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.len() == 42 {
            return Err(Self::Err::InvalidLength);
        }
        let _ = bech32::decode(s)?;
        let mut bytes = [0; 42];
        bytes.copy_from_slice(s.as_bytes());
        Ok(Address42(bytes))
    }
}

struct Address42Visitor;

impl<'b> serde::de::Visitor<'b> for Address42Visitor {
    type Value = Address42;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string with len {}", 42)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if !v.len() == 42 {
            return Err(E::invalid_length(v.len(), &self));
        }
        let _ = bech32::decode(v).map_err(|e| E::custom(e))?;
        let mut bytes = [0; 42];
        bytes.copy_from_slice(v.as_bytes());
        Ok(Address42(bytes))
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }
}

impl ::serde::Serialize for Address42 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(
            &String::from_utf8(self.0.to_vec()).map_err(|e| S::Error::custom(&e.to_string()))?,
        )
    }
}
impl<'de> ::serde::Deserialize<'de> for Address42 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(Address42Visitor)
    }
}

impl PartialEq for Address42 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Address42 {}

pub fn get_address_from_pub_key(pub_key: PublicKey, network: Network) -> Address42 {
    let key = pub_key.hash();
    let checksum = &key[12..];
    let address: String = bech32::encode(network.hrp(), checksum.to_base32(), Variant::Bech32m)
        .expect("error creating account id");
    let mut raw_address = [0; 42];
    raw_address.copy_from_slice(address.as_bytes());
    Address42(raw_address)
}
pub fn get_eth_address_from_pub_key(pub_key: PublicKey) -> H160 {
    let pubkey_bytes = pub_key.to_bytes();
    let key = keccak256(&pubkey_bytes[1..]).to_fixed_bytes();
    let checksum = &key[12..];
    H160::from_slice(checksum)
}

#[cfg(test)]
mod tests {
    use crate::account::Address42;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct CAccount {
        account_id: Address42,
        balance: i32,
    }

    #[test]
    fn test_valid_ser() {
        let account = CAccount {
            account_id: Address42([
                117, 99, 104, 49, 121, 50, 114, 50, 51, 103, 55, 53, 99, 119, 56, 118, 48, 116,
                101, 119, 100, 50, 104, 50, 106, 118, 54, 97, 118, 117, 121, 101, 50, 122, 121,
                117, 119, 112, 101, 56, 106, 51,
            ]),
            balance: 0,
        };
        let raw_json = serde_json::to_string_pretty(&account).unwrap();
        println!("{}", raw_json);

        let d_account: CAccount = serde_json::from_str(&raw_json).unwrap();
        println!("{:#?}", d_account);
        println!("{:?}", d_account.account_id);
        println!("{}", d_account.account_id);
    }
}
