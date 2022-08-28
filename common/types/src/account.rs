use std::hash::Hash;
use anyhow::{anyhow, Result};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use std::{u128, u64};
use bytes::{Buf, BufMut};
use prost::{DecodeError, Message};
use prost::encoding::{DecodeContext, WireType};
use serde::ser::Error;

use serde::{Deserialize, Serialize};
use bech32::{ToBase32, Variant};

use codec::{Decodable, Encodable};
use crypto::ecdsa::{PublicKey, SecretKey, Signature};
use crypto::{keccak256};
use hex::{FromHex, ToHex};
use primitive_types::{H160, H256};
use crate::network::{ALPHA_HRP, MAINNET_HRP, Network, TESTNET_HRP};

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    #[serde(with = "hex")]
    pub free_balance: u128,
    #[serde(with = "hex")]
    pub reserve_balance: u128,
    #[serde(with = "hex")]
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

impl prost::Message for AccountState {
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut, Self: Sized {
        prost::encoding::string::encode(1, &self.free_balance.encode_hex(), buf);
        prost::encoding::string::encode(2, &self.reserve_balance.encode_hex(), buf);
        prost::encoding::string::encode(3, &self.nonce.encode_hex(), buf);
    }

    fn merge_field<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B, ctx: DecodeContext) -> std::result::Result<(), DecodeError> where B: Buf, Self: Sized {
        const STRUCT_NAME: &'static str = "AccountState";
        match tag {
            1 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type,&mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "free_balance");
                        error
                    },
                )?;
                self.free_balance = u128::from_hex(&raw_value).map_err(
                    |error| {
                        let mut error = DecodeError::new(error.to_string());
                        error.push(STRUCT_NAME, "free_balance");
                        error
                    },
                )?;
                Ok(())
            }
            2 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type,&mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "reserve_balance");
                        error
                    },
                )?;
                self.reserve_balance = u128::from_hex(&raw_value).map_err(
                    |error| {
                        let mut error = DecodeError::new(error.to_string());
                        error.push(STRUCT_NAME, "reserve_balance");
                        error
                    },
                )?;
                Ok(())
            }
            3 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type,&mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "nonce");
                        error
                    },
                )?;
                self.nonce = u64::from_str(&raw_value).map_err(
                    |error| {
                        let mut error = DecodeError::new(error.to_string());
                        error.push(STRUCT_NAME, "nonce");
                        error
                    },
                )?;
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0 + prost::encoding::string::encoded_len(1, &self.free_balance.encode_hex())
        + prost::encoding::string::encoded_len(2, &self.reserve_balance.encode_hex())
        + prost::encoding::string::encoded_len(3, &self.nonce.encode_hex())
    }

    fn clear(&mut self) {
    }
}

impl Encodable for AccountState {
    fn encode(&self) -> Result<Vec<u8>> {
       Ok(self.encode_to_vec())
    }
}

impl Decodable for AccountState {
    fn decode(buf: &[u8]) -> Result<Self> {
        <AccountState as prost::Message>::decode(buf).map_err(|e|anyhow!(e))
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

#[derive(Copy,Clone)]
pub struct Address42(pub [u8; 42]);

impl Debug  for Address42 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
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

    fn hrp(&self) -> String {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp,_,_)) => hrp,
            Err(_) => String::new()
        }
    }
    pub fn is_valid(&self) -> bool {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp,_,_)) => ALPHA_HRP.eq(&hrp) || TESTNET_HRP.eq(&hrp) || MAINNET_HRP.eq(&hrp),
            Err(_) => false
        }
    }

    pub fn network(&self) -> Option<Network> {
        match self.hrp().as_str() {
            MAINNET_HRP => Some(Network::Mainnet),
            TESTNET_HRP => Some(Network::Testnet),
            ALPHA_HRP => Some(Network::Alphanet),
            _ => None
        }
    }

    pub fn to_address20(&self) -> Option<H160> {
        match bech32::decode(&String::from_utf8_lossy(&self.0)).and_then(|(_,address_32,_)|{
            bech32::convert_bits(&address_32, 5, 8, false)
        }) {
            Ok(address) => Some(H160::from_slice(&address)),
            Err(_) => None
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
        let mut bytes = [0;42];
        bytes.copy_from_slice(s.as_bytes());
        Ok(Address42(bytes))
    }
}

struct Address42Visitor;

impl<'b> serde::de::Visitor<'b> for Address42Visitor {
    type Value = Address42;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a string with len {}",
            42
        )
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if !v.len() == 42 {
            return Err(E::invalid_length(v.len(), &self));
        }
        let _ = bech32::decode(v).map_err(|e| E::custom(e))?;
        let mut bytes = [0;42];
        bytes.copy_from_slice(v.as_bytes());
        Ok(Address42(bytes))
    }

    fn visit_string<E:serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }
}

impl ::serde::Serialize for Address42 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
    {
        serializer.serialize_str(&String::from_utf8(self.0.to_vec()).map_err(|e| S::Error::custom(&e.to_string()))?)
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
    let address : String = bech32::encode(network.hrp(), checksum.to_base32(), Variant::Bech32m).expect("error creating account id");
    let mut raw_address = [0;42];
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
    use serde::{Serialize,Deserialize};
    use crate::account::Address42;

    #[derive(Serialize,Deserialize, Debug)]
    struct CAccount {
        account_id : Address42,
        balance : i32
    }

    #[test]
    fn test_valid_ser() {
        let account = CAccount {
            account_id: Address42([117, 99, 104, 49, 121, 50, 114, 50, 51, 103, 55, 53, 99, 119, 56, 118, 48, 116, 101, 119, 100, 50, 104, 50, 106, 118, 54, 97, 118, 117, 121, 101, 50, 122, 121, 117, 119, 112, 101, 56, 106, 51]),
            balance: 0
        };
        let raw_json = serde_json::to_string_pretty(&account).unwrap();
        println!("{}", raw_json);

        let d_account : CAccount = serde_json::from_str(&raw_json).unwrap();
        println!("{:#?}", d_account);
        println!("{:?}", d_account.account_id.0);
    }
}