use std::cmp::Ordering;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::u128;

use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use prost_types::Any;
use serde::{Deserialize, Serialize};

use codec::{Decodable, Encodable};
use crypto::ecdsa::{PublicKey, Signature};
use crypto::{keccak256, sha256, SHA256};
use hex::{FromHex, ToHex};
use primitive_types::{H160, H256, U128};

use crate::account::{get_address_from_pub_key, Account};
use crate::network::Network;
use crate::{cache, Hash};

#[derive(Serialize, Deserialize, PartialEq, Default, Debug, Clone)]
pub struct PaymentTx {
    pub to: H160,
    pub amount: U128,
}

impl Message for PaymentTx {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::string::encode(1, &self.to.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(2, &self.amount.encode_hex(), buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        const STRUCT_NAME: &'static str = "PaymentTx";
        match tag {
            1 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "to");
                        error
                    },
                )?;
                self.to = H160::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "to");
                    error
                })?;
                Ok(())
            }
            2 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "amount");
                        error
                    },
                )?;
                self.amount = U128::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "amount");
                    error
                })?;
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0 + prost::encoding::string::encoded_len(1, &self.to.as_fixed_bytes().encode_hex())
            + prost::encoding::string::encoded_len(2, &self.amount.encode_hex())
    }

    fn clear(&mut self) {}
}

#[derive(Clone, PartialEq, Serialize, Deserialize, ::prost::Message)]
pub struct AnyType {
    /// used with implementation specific semantics.
    #[prost(string, tag = "1")]
    pub type_info: ::prost::alloc::string::String,
    /// Must be a hex encoded valid serialized protocol buffer of the above specified type.
    #[prost(string, tag = "2")]
    pub value: ::prost::alloc::string::String,
}

#[derive(Serialize, Deserialize, PartialEq, prost::Message, Clone)]
pub struct ApplicationCallTx {
    #[prost(uint32, tag = "1")]
    pub app_id: u32,
    #[prost(message, tag = "2")]
    pub args: Option<AnyType>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, prost::Oneof)]
#[serde(rename_all = "snake_case")]
pub enum TransactionData {
    #[prost(message, tag = "5")]
    Payment(PaymentTx),
    #[prost(message, tag = "6")]
    Call(ApplicationCallTx),
    #[prost(string, tag = "7")]
    #[serde(rename = "raw")]
    RawData(String),
}

impl Default for TransactionData {
    fn default() -> Self {
        Self::RawData(Default::default())
    }
}

const STRUCT_NAME: &'static str = "Transaction";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone)]
pub struct UnsignedTransaction {
    #[serde(with = "hex")]
    pub nonce: u64,
    pub chain_id: u16,
    pub genesis_hash: H256,
    pub fee: U128,
    #[serde(flatten)]
    pub data: TransactionData,
}

pub struct TransactionBuilder<'a> {
    tx: UnsignedTransaction,
    account: &'a Account,
}

impl<'a> TransactionBuilder<'a> {
    pub fn with_signer(account: &'a Account) -> Result<Self> {
        let mut tx = UnsignedTransaction::default();
        tx.chain_id = account
            .address
            .network()
            .ok_or_else(|| anyhow::anyhow!("network not specified on signer"))?
            .chain_id();
        Ok(Self {
            tx,
            account,
        })
    }

    pub fn chain_id(&mut self, chain_id: u16) -> &mut Self {
        self.tx.chain_id = chain_id;
        self
    }

    pub fn nonce(&mut self, nonce: u64) -> &mut Self {
        self.tx.nonce = nonce;
        self
    }

    pub fn genesis_hash(&mut self, hash: H256) -> &mut Self {
        self.tx.genesis_hash = hash;
        self
    }

    pub fn fee(&mut self, fee: u128) -> &mut Self {
        self.tx.fee = fee.into();
        self
    }

    pub fn call(&'a mut self) -> AppCallTransactionBuilder<'a> {
        self.tx.data = TransactionData::Call(ApplicationCallTx::default());
        AppCallTransactionBuilder {
            inner: self
        }
    }

    pub fn transfer(&'a mut self) -> TransferTransactionBuilder<'a> {
        self.tx.data = TransactionData::Payment(PaymentTx::default());
        TransferTransactionBuilder {
            inner: self
        }
    }

    pub fn build(&'a mut self) -> Result<SignedTransaction> {
        self.account.sign(self.tx.sig_hash().as_bytes()).and_then(|sig| {
            SignedTransaction::new(sig, self.tx.clone())
        })
    }
}

pub struct TransferTransactionBuilder<'a> {
    inner: &'a mut TransactionBuilder<'a>
}

impl<'a> TransferTransactionBuilder<'a> {
    pub fn to(&mut self, to: H160) -> &mut Self {
        match &mut self.inner.tx.data {
            TransactionData::Payment(pmt) => pmt.to = to,
            _ => {}
        }
        self
    }

    pub fn amount(&mut self, amount: u128) -> &mut Self {
        match &mut self.inner.tx.data {
            TransactionData::Payment(pmt) => {
                pmt.amount = amount.into();
            }
            _ => {}
        }
        self
    }

    pub fn build(&mut self) -> Result<SignedTransaction> {
        self.inner.account.sign(self.inner.tx.sig_hash().as_bytes()).and_then(|sig| {
            SignedTransaction::new(sig, self.inner.tx.clone())
        })
    }
}
pub struct AppCallTransactionBuilder<'a> {
    inner: &'a mut TransactionBuilder<'a>
}

impl<'a> AppCallTransactionBuilder<'a> {
    pub fn app_id(&mut self, app_id: u32) -> &mut Self {
        match &mut self.inner.tx.data {
            TransactionData::Call(call) => call.app_id = app_id,
            _ => {}
        }
        self
    }

    pub fn args(&mut self, args: Option<AnyType>) -> &mut Self {
        match &mut self.inner.tx.data {
            TransactionData::Call(call) => {
                call.args = args;
            }
            _ => {}
        }
        self
    }

    pub fn build(&mut self) -> Result<SignedTransaction> {
        self.inner.account.sign(self.inner.tx.sig_hash().as_bytes()).and_then(|sig| {
            SignedTransaction::new(sig, self.inner.tx.clone())
        })
    }
}
pub struct RawDataTransactionBuilder<'a> {
    tx: UnsignedTransaction,
    account: &'a Account,
}

impl<'a> RawDataTransactionBuilder<'a> {
    pub fn with_raw(&mut self, data: String) -> &mut Self {
        match &mut self.tx.data {
            TransactionData::RawData(raw) => *raw = data,
            _ => {}
        }
        self
    }

    pub fn build(self) -> Result<SignedTransaction> {
        self.account.sign(self.tx.sig_hash().as_bytes()).and_then(|sig| {
            SignedTransaction::new(sig, self.tx)
        })
    }
}

impl prost::Message for UnsignedTransaction {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::string::encode(1, &self.nonce.encode_hex(), buf);
        prost::encoding::string::encode(2, &self.chain_id.encode_hex(), buf);
        prost::encoding::string::encode(3, &self.genesis_hash.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(4, &self.fee.encode_hex(), buf);
        self.data.encode(buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        match tag {
            1 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "nonce");
                        error
                    },
                )?;
                self.nonce = u64::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "nonce");
                    error
                })?;
                Ok(())
            }
            2 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "chain_id");
                        error
                    },
                )?;
                self.chain_id = u16::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "chain_id");
                    error
                })?;
                Ok(())
            }
            3 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "genesis_hash");
                        error
                    },
                )?;
                self.genesis_hash = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "genesis_hash");
                    error
                })?;
                Ok(())
            }
            4 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "fee");
                        error
                    },
                )?;
                self.fee = U128::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "fee");
                    error
                })?;
                Ok(())
            }
            5 | 6 | 7 => {
                let mut value: Option<TransactionData> = None;
                TransactionData::merge(&mut value, tag, wire_type, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "data");
                        error
                    },
                )?;

                match value {
                    None => {
                        self.data = TransactionData::RawData(Default::default())
                    }
                    Some(data) => self.data = data,
                }

                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0 + prost::encoding::string::encoded_len(1, &self.nonce.encode_hex())
            + prost::encoding::string::encoded_len(2, &self.chain_id.encode_hex())
            + prost::encoding::string::encoded_len(
                3,
                &self.genesis_hash.as_fixed_bytes().encode_hex(),
            )
            + prost::encoding::string::encoded_len(4, &self.fee.encode_hex())
            + self.data.encoded_len()
    }

    fn clear(&mut self) {}
}

impl UnsignedTransaction {
    pub fn sig_hash(&self) -> H256 {
        let mut pack = self.pack();
        sha256(pack)
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut pack = Vec::new();
        pack.extend_from_slice(&self.nonce.to_be_bytes());
        pack.extend_from_slice(&self.chain_id.to_be_bytes());
        pack.extend_from_slice(&self.genesis_hash.as_bytes());
        pack.extend_from_slice(&self.fee.to_be_bytes());
        match &self.data {
            TransactionData::Payment(pmt) => {
                pack.extend_from_slice(&1u32.to_be_bytes());
                pack.extend_from_slice(&pmt.amount.to_be_bytes());
                pack.extend_from_slice(&pmt.to.as_bytes());
            }
            TransactionData::Call(call) => {
                pack.extend_from_slice(&2u32.to_be_bytes());
                pack.extend_from_slice(&call.app_id.to_be_bytes());
                match &call.args {
                    None => {}
                    Some(args) => {
                        pack.extend_from_slice(&args.value.as_bytes());
                    }
                }
            }
            TransactionData::RawData(raw) => {
                pack.extend_from_slice(&3u32.to_be_bytes());
                pack.extend_from_slice(raw.as_bytes())
            },
        }
        pack
    }
}

#[derive(Serialize, Deserialize, PartialEq,  Clone, Debug, Default)]
pub struct TransactionList {
    pub txs: Vec<Arc<SignedTransaction>>,
}

impl Message for TransactionList {
    fn encode_raw<B>(&self, buf: &mut B) where B: BufMut, Self: Sized {
        for msg in &self.txs {
            prost::encoding::message::encode(1u32, msg.as_ref(), buf);
        }
    }

    fn merge_field<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B, ctx: DecodeContext) -> std::result::Result<(), DecodeError> where B: Buf, Self: Sized {
        const STRUCT_NAME: &'static str = "TransactionList";
        match tag {
            1 => {
                let mut value : Vec<SignedTransaction> = Vec::new();
                prost::encoding::message::merge_repeated(wire_type, &mut value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "txs");
                        error
                    },
                )?;
                for tx in value {
                    self.txs.push(Arc::new(tx))
                }
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0 + prost::encoding::key_len(1) * self.txs.len()
            + self.txs
            .iter()
            .map(|tx| tx.as_ref())
            .map(Message::encoded_len)
            .map(|len| len + prost::encoding::encoded_len_varint(len as u64))
            .sum::<usize>()
    }

    fn clear(&mut self) {
        self.txs.clear()
    }
}

impl TransactionList {
    pub fn new(txs: Vec<Arc<SignedTransaction>>) -> Self {
        Self { txs }
    }
}

impl AsRef<Vec<Arc<SignedTransaction>>> for TransactionList {
    fn as_ref(&self) -> &Vec<Arc<SignedTransaction>> {
        &self.txs
    }
}

impl AsMut<Vec<Arc<SignedTransaction>>> for TransactionList {
    fn as_mut(&mut self) -> &mut Vec<Arc<SignedTransaction>> {
        &mut self.txs
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    #[serde(flatten)]
    tx: UnsignedTransaction,
    //tag 7
    r: H256,
    //tag 8
    s: H256,
    //tag 9
    #[serde(with = "hex")]
    v: u8,
    //caches
    #[serde(skip)]
    hash: Arc<RwLock<Option<Hash>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<H160>>>,
}

impl PartialEq for SignedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash().eq(&other.hash())
    }
}

impl Eq for SignedTransaction {}

impl PartialOrd for SignedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.hash().partial_cmp(&other.hash())
    }
}

impl Ord for SignedTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash().cmp(&other.hash())
    }
}

impl std::hash::Hash for SignedTransaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.hash())
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, tx: UnsignedTransaction) -> Result<Self> {
        let (r, s, v) = signature.rsv();
        Ok(Self {
            tx,
            r,
            s,
            v,
            hash: Arc::new(Default::default()),
            from: Arc::new(Default::default()),
        })
    }

    pub fn hash(&self) -> [u8; 32] {
        cache(&self.hash, || self.tx.sig_hash().to_fixed_bytes())
    }

    pub fn hash_256(&self) -> H256 {
        H256::from(self.hash())
    }

    pub fn signature(&self) -> [u8; 65] {
        let sig = Signature::from_rsv((self.r, self.s, self.v)).unwrap();
        sig.to_bytes()
    }

    pub fn nonce(&self) -> u64 {
        self.tx.nonce
    }
    pub fn sender(&self) -> H160 {
        self.from()
    }

    pub fn to(&self) -> H160 {
        match &self.tx.data {
            TransactionData::Payment(pmt) => pmt.to,
            TransactionData::Call(_) => H160::default(),
            TransactionData::RawData(_) => H160::default(),
        }
    }

    pub fn origin(&self) -> H160 {
        self.from()
    }

    pub fn raw_origin(&self) -> Result<PublicKey> {
        let signature = Signature::from_rsv((&self.r, &self.s, self.v))?;
        let pub_key = signature.recover_public_key(&self.sig_hash()?)?;
        Ok(pub_key)
    }

    pub fn from(&self) -> H160 {
        cache(&self.from, || {
            Signature::from_rsv((&self.r, &self.s, self.v))
                .map_err(|e| anyhow::anyhow!(e))
                .and_then(|signature| {
                    self.sig_hash().and_then(|sig_hash| {
                        signature
                            .recover_public_key(&sig_hash)
                            .map_err(|e| anyhow::anyhow!(e))
                            .map(|key| {
                                get_address_from_pub_key(key, Network::from_u32(self.tx.chain_id))
                                    .to_address20()
                                    .unwrap()
                            })
                    })
                })
                .unwrap_or_default()
        })
    }

    pub fn fees(&self) -> u128 {
        self.tx.fee.as_u128()
    }

    pub fn price(&self) -> u128 {
        match &self.tx.data {
            TransactionData::Payment(p) => p.amount.as_u128(),
            TransactionData::Call(_) => 10_000_000,
            TransactionData::RawData(s) => (s.len() as u128) * 1000,
        }
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let raw = self.tx.sig_hash();
        Ok(raw.to_fixed_bytes())
    }

    pub fn size(&self) -> u64 {
        self.encoded_len() as u64
    }
}

impl prost::Message for SignedTransaction {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        self.tx.encode_raw(buf);
        prost::encoding::string::encode(8, &self.r.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(9, &self.s.as_fixed_bytes().encode_hex(), buf);
        prost::encoding::string::encode(10, &self.v.encode_hex(), buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        match tag {
            1..=7 => self.tx.merge_field(tag, wire_type, buf, ctx),
            8 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "r");
                        error
                    },
                )?;
                self.r = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "r");
                    error
                })?;
                Ok(())
            }
            9 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "s");
                        error
                    },
                )?;
                self.s = H256::from_str(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "s");
                    error
                })?;
                Ok(())
            }
            10 => {
                let mut raw_value = String::new();
                prost::encoding::string::merge(wire_type, &mut raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "v");
                        error
                    },
                )?;
                self.v = u8::from_hex(&raw_value).map_err(|error| {
                    let mut error = DecodeError::new(error.to_string());
                    error.push(STRUCT_NAME, "v");
                    error
                })?;
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        0 + self.tx.encoded_len()
            + prost::encoding::string::encoded_len(8, &self.r.as_fixed_bytes().encode_hex())
            + prost::encoding::string::encoded_len(9, &self.s.as_fixed_bytes().encode_hex())
            + prost::encoding::string::encoded_len(10, &self.v.encode_hex())
    }

    fn clear(&mut self) {}
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::account::{get_address_from_pub_key, Account};
    use crate::network::Network;
    use crate::tx::{AnyType, UnsignedTransaction, TransactionBuilder};
    use crypto::ecdsa::Keypair;
    use primitive_types::{H160, H256};
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;
    use hex::ToHex;
    use prost::Message;
    use crate::prelude::{SignedTransaction, TransactionList};

    pub fn create_account(network: Network) -> Account {
        let mut csprng = ChaCha20Rng::from_entropy();
        let keypair = Keypair::generate(&mut csprng);
        let secret = H256::from(keypair.secret.to_bytes());
        let address = get_address_from_pub_key(keypair.public, network);
        Account { address, secret }
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, prost::Message, Clone)]
    pub struct TransferToken {
        #[prost(uint32, tag = "1")]
        pub token_id: u32,
        #[prost(string, tag = "2")]
        pub from: String,
        #[prost(string, tag = "3")]
        pub to: String,
        #[prost(string, tag = "4")]
        pub amount: String,
    }


    #[test]
    fn test_encoding_and_decoding() {
        let genesis = H256::random();
        let user1 = create_account(Network::Testnet);
        let user2 = create_account(Network::Testnet);
        let mut txs = TransactionList::new(Vec::new());
        let tx = TransactionBuilder::with_signer(&user1)
            .unwrap()
            .nonce(1)
            .fee(100_000)
            .genesis_hash(genesis)
            .transfer()
            .to(user2.address.to_address20().unwrap())
            .amount(1_000_000)
            .build().unwrap();
        txs.as_mut().push(Arc::new(tx));
         let tx = TransactionBuilder::with_signer(&user1)
            .unwrap()
            .nonce(1)
            .fee(100_000)
            .genesis_hash(genesis)
             .call()
             .app_id(20)
             .args(Some(AnyType {
                 type_info: "TransferToken".to_string(),
                 value: {
                     let t = TransferToken {
                         token_id: 10,
                         from: H160::zero().as_fixed_bytes().encode_hex(),
                         to: H160::zero().as_fixed_bytes().encode_hex(),
                         amount: 100000_u128.encode_hex(),
                     };
                     hex::encode(t.encode_to_vec(), false)
                 }
             }))
            .build().unwrap();
        txs.as_mut().push(Arc::new(tx));
         let tx = TransactionBuilder::with_signer(&user1)
            .unwrap()
            .nonce(1)
            .fee(100_000)
            .genesis_hash(genesis)
            .build().unwrap();
        txs.as_mut().push(Arc::new(tx));
        println!("{}",serde_json::to_string_pretty(&txs).unwrap());
    }

    #[test]
    fn test_encoding_and_decoding_2() {
        let a = "0x0a0330783112033078311a42307830303030303030303030303030303030303030\
        303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030\
        2207307831383661302a350a2a3078366266383336363338653164653637343133626331353732623463623161\
        656138346230383336631207307866343234304242307839623139323237373164353034353232633336383765\
        3933383432323733363330383665303333623061306434666430353137333730323063653137326363354a4230\
        783435386132383630626437393234326239363466383838643934633666663261643865396632656261613736\
        62623465353236663333616665646562306663635203307830";
        let tx = SignedTransaction::decode(hex::decode(a).unwrap().as_slice()).unwrap();
        let b = hex::encode(tx.encode_to_vec(), false);
        assert_eq!(a,b)
    }

}
