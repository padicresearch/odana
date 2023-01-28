use std::cmp::Ordering;
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use serde::{Deserialize, Serialize};

use crypto::ecdsa::{PublicKey, Signature};
use crypto::sha256;
use primitive_types::{Address, H256};

use crate::account::{get_address_from_app_id, get_address_from_pub_key, Account};
use crate::network::Network;
use crate::{cache, Addressing, Hash};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TransactionStatus {
    Confirmed = 0,
    Pending = 1,
    Queued = 2,
    NotFound = 3,
}
impl TransactionStatus {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TransactionStatus::Confirmed => "Confirmed",
            TransactionStatus::Pending => "Pending",
            TransactionStatus::Queued => "Queued",
            TransactionStatus::NotFound => "NotFound",
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default, Debug, Clone)]
pub struct PaymentTx {
    pub to: Address,
}

impl Message for PaymentTx {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::bytes::encode(1, &self.to, buf);
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
        const STRUCT_NAME: &str = "PaymentTx";
        match tag {
            1 => prost::encoding::bytes::merge(wire_type, &mut self.to, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "to");
                    error
                },
            ),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::bytes::encoded_len(1, &self.to)
    }

    fn clear(&mut self) {}
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct ApplicationCallTx {
    #[prost(bytes, tag = "1")]
    pub app_id: Vec<u8>,
    #[prost(bytes, tag = "2")]
    pub args: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct CreateApplicationTx {
    #[prost(bytes, tag = "1")]
    pub app_id: Vec<u8>,
    #[prost(bytes, tag = "2")]
    pub binary: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct UpdateApplicationTx {
    #[prost(bytes, tag = "1")]
    pub app_id: Vec<u8>,
    #[prost(bytes, tag = "2")]
    pub binary: Vec<u8>,
    #[prost(bool, tag = "3")]
    pub migrate: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, prost::Oneof)]
#[serde(rename_all = "snake_case")]
pub enum TransactionData {
    #[prost(message, tag = "6")]
    Payment(PaymentTx),
    #[prost(message, tag = "7")]
    Call(ApplicationCallTx),
    #[prost(message, tag = "8")]
    Create(CreateApplicationTx),
    #[prost(message, tag = "9")]
    Update(UpdateApplicationTx),
    #[prost(string, tag = "10")]
    #[serde(rename = "raw")]
    RawData(String),
}

impl Default for TransactionData {
    fn default() -> Self {
        Self::RawData(Default::default())
    }
}

const STRUCT_NAME: &str = "Transaction";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
pub struct Transaction {
    pub nonce: u64,
    pub chain_id: u32,
    pub genesis_hash: H256,
    pub fee: u64,
    pub value: u64,
    #[serde(flatten)]
    pub data: TransactionData,
}

impl prost::Message for Transaction {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::uint64::encode(1, &self.nonce, buf);
        prost::encoding::uint32::encode(2, &self.chain_id, buf);
        prost::encoding::bytes::encode(3, &self.genesis_hash, buf);
        prost::encoding::uint64::encode(4, &self.fee, buf);
        prost::encoding::uint64::encode(5, &self.value, buf);
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
                let raw_value = &mut self.nonce;
                prost::encoding::uint64::merge(wire_type, raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "nonce");
                        error
                    },
                )
            }
            2 => {
                let raw_value = &mut self.chain_id;
                prost::encoding::uint32::merge(wire_type, raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "chain_id");
                        error
                    },
                )
            }
            3 => {
                let raw_value = &mut self.genesis_hash;
                prost::encoding::bytes::merge(wire_type, raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "genesis_hash");
                        error
                    },
                )
            }
            4 => {
                let raw_value = &mut self.fee;
                prost::encoding::uint64::merge(wire_type, raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "fee");
                        error
                    },
                )
            }
            5 => {
                let raw_value = &mut self.value;
                prost::encoding::uint64::merge(wire_type, raw_value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "value");
                        error
                    },
                )
            }
            6 | 7 | 8 | 9 | 10 => {
                let mut value: Option<TransactionData> = None;
                TransactionData::merge(&mut value, tag, wire_type, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "data");
                        error
                    },
                )?;

                match value {
                    None => self.data = TransactionData::RawData(Default::default()),
                    Some(data) => self.data = data,
                }

                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::uint64::encoded_len(1, &self.nonce)
            + prost::encoding::uint32::encoded_len(2, &self.chain_id)
            + prost::encoding::bytes::encoded_len(3, &self.genesis_hash)
            + prost::encoding::uint64::encoded_len(4, &self.fee)
            + self.data.encoded_len()
    }

    fn clear(&mut self) {}
}

impl Transaction {
    pub fn sig_hash(&self) -> H256 {
        let pack = self.pack();
        sha256(pack)
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut pack = Vec::new();
        pack.extend_from_slice(&self.nonce.to_be_bytes());
        pack.extend_from_slice(&self.chain_id.to_be_bytes());
        pack.extend_from_slice(self.genesis_hash.as_bytes());
        pack.extend_from_slice(&self.fee.to_be_bytes());
        pack.extend_from_slice(&self.value.to_be_bytes());
        match &self.data {
            TransactionData::Payment(v) => {
                pack.extend_from_slice(&1u32.to_be_bytes());
                pack.extend_from_slice(v.to.as_bytes());
            }
            TransactionData::Call(v) => {
                pack.extend_from_slice(&2u32.to_be_bytes());
                pack.extend_from_slice(&v.app_id);
                pack.extend_from_slice(&v.args);
            }
            TransactionData::Create(v) => {
                pack.extend_from_slice(&3u32.to_be_bytes());
                pack.extend_from_slice(&v.app_id);
                pack.extend_from_slice(&v.binary);
            }
            TransactionData::Update(v) => {
                pack.extend_from_slice(&4u32.to_be_bytes());
                pack.extend_from_slice(&v.app_id);
                pack.extend_from_slice(&(v.migrate as u8).to_be_bytes());
                pack.extend_from_slice(&v.binary);
            }
            TransactionData::RawData(v) => {
                pack.extend_from_slice(&5u32.to_be_bytes());
                pack.extend_from_slice(v.as_bytes())
            }
        }
        pack
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
pub struct TransactionList {
    pub txs: Vec<Arc<SignedTransaction>>,
}

impl Message for TransactionList {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for msg in &self.txs {
            prost::encoding::message::encode(1u32, msg.as_ref(), buf);
        }
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
        const STRUCT_NAME: &str = "TransactionList";
        match tag {
            1 => {
                let mut value: Vec<SignedTransaction> = Vec::new();
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
        prost::encoding::key_len(1) * self.txs.len()
            + self
                .txs
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
    tx: Transaction,
    //tag 7
    r: H256,
    //tag 8
    s: H256,
    //tag 9
    #[serde(with = "hex")]
    v: u8,
    //caches
    #[serde(skip)]
    hash: H256,
    #[serde(skip)]
    from: Address,
    #[serde(skip)]
    to: Address,
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
        state.write(&self.hash().as_bytes())
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, tx: Transaction) -> Result<Self> {
        let (r, s, v) = signature.rsv();

        let from = signature
            .recover_public_key(tx.sig_hash().as_bytes())
            .map_err(|e| anyhow::anyhow!(e))
            .map(|key| get_address_from_pub_key(key, Network::from_chain_id(tx.chain_id)))?;

        let to = match &tx.data {
            TransactionData::Payment(PaymentTx { to, .. }) => *to,
            TransactionData::Call(ApplicationCallTx { app_id, .. }) => {
                Address::from_slice(app_id).map_err(|_| anyhow!("invalid address"))?
            }
            TransactionData::RawData(_) => Default::default(),
            TransactionData::Create(CreateApplicationTx { app_id, .. }) => {
                Address::from_slice(app_id).map_err(|_| anyhow!("invalid address"))?
            }
            TransactionData::Update(UpdateApplicationTx { app_id, .. }) => {
                Address::from_slice(app_id).map_err(|_| anyhow!("invalid address"))?
            }
        };

        let hash = tx.sig_hash();

        Ok(Self {
            tx,
            r,
            s,
            v,
            hash,
            from,
            to,
        })
    }

    pub fn hash(&self) -> H256 {
        self.hash
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
    pub fn sender(&self) -> Address {
        self.from()
    }

    pub fn to(&self) -> Address {
        self.to
    }

    pub fn origin(&self) -> Address {
        self.from()
    }

    pub fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn raw_origin(&self) -> Result<PublicKey> {
        let signature = Signature::from_rsv((&self.r, &self.s, self.v))?;
        let pub_key = signature.recover_public_key(&self.sig_hash()?)?;
        Ok(pub_key)
    }

    pub fn from(&self) -> Address {
        self.from
    }

    pub fn fees(&self) -> u64 {
        self.tx.fee
    }

    pub fn price(&self) -> u64 {
        self.tx.value
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
        prost::encoding::bytes::encode(10, &self.r, buf);
        prost::encoding::bytes::encode(11, &self.s, buf);
        prost::encoding::bytes::encode(12, &self.v, buf);
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
            1..=9 => self.tx.merge_field(tag, wire_type, buf, ctx),
            10 => prost::encoding::bytes::merge(wire_type, &mut self.r, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "r");
                    error
                },
            ),
            11 => prost::encoding::bytes::merge(wire_type, &mut self.s, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "s");
                    error
                },
            ),
            12 => prost::encoding::bytes::merge(wire_type, &mut self.v, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "v");
                    error
                },
            ),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        self.tx.encoded_len()
            + prost::encoding::bytes::encoded_len(10, &self.r)
            + prost::encoding::bytes::encoded_len(11, &self.s)
            + prost::encoding::bytes::encoded_len(12, &self.v)
    }

    fn clear(&mut self) {}
}
