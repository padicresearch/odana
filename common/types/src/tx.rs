use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut};
use codec::{impl_codec_using_prost, Decodable, Encodable};
use parking_lot::RwLock;
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

use crypto::ecdsa::{PublicKey, Signature};
use crypto::keccak256;
use primitive_types::{Address, H256};

use crate::account::get_address_from_pub_key;
use crate::network::Network;
use crate::prelude::get_address_from_package_name;
use crate::{cache, Addressing};

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
    #[prost(required, message, tag = "1")]
    pub app_id: Address,
    #[prost(bytes, tag = "2")]
    #[serde(with = "hex")]
    pub args: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct CreateApplicationTx {
    #[prost(required, string, tag = "1")]
    pub package_name: String,
    #[prost(bytes, tag = "2")]
    #[serde(with = "hex")]
    pub binary: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct UpdateApplicationTx {
    #[prost(required, message, tag = "1")]
    pub app_id: Address,
    #[prost(bytes, tag = "2")]
    #[serde(with = "hex")]
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
    #[prost(bytes, tag = "10")]
    #[serde(rename = "raw")]
    RawData(Vec<u8>),
}

impl Default for TransactionData {
    fn default() -> Self {
        Self::RawData(Default::default())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, prost::Message)]
pub struct Transaction {
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
    #[prost(uint32, tag = "2")]
    pub chain_id: u32,
    #[prost(required, message, tag = "3")]
    pub genesis_hash: H256,
    #[prost(uint64, tag = "4")]
    pub fee: u64,
    #[prost(uint64, tag = "5")]
    pub value: u64,
    #[serde(flatten)]
    #[prost(oneof = "TransactionData", tags = "6, 7, 8, 9, 10")]
    pub data: Option<TransactionData>,
}

impl Transaction {
    pub fn sig_hash(&self) -> H256 {
        let pack = self.pack();
        keccak256(pack)
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut pack = Vec::new();
        pack.extend_from_slice(&self.nonce.to_be_bytes());
        pack.extend_from_slice(&self.chain_id.to_be_bytes());
        pack.extend_from_slice(self.genesis_hash.as_bytes());
        pack.extend_from_slice(&self.fee.to_be_bytes());
        pack.extend_from_slice(&self.value.to_be_bytes());

        let Some(data) = self.data.as_ref() else {
            return pack
        };
        match data {
            TransactionData::Payment(v) => {
                pack.extend_from_slice(&1u32.to_be_bytes());
                pack.extend_from_slice(v.to.as_bytes());
            }
            TransactionData::Call(v) => {
                pack.extend_from_slice(&2u32.to_be_bytes());
                pack.extend_from_slice(&v.app_id.as_bytes());
                pack.extend_from_slice(&v.args);
            }
            TransactionData::Create(v) => {
                pack.extend_from_slice(&3u32.to_be_bytes());
                pack.extend_from_slice(v.package_name.as_bytes());
                pack.extend_from_slice(&v.binary);
            }
            TransactionData::Update(v) => {
                pack.extend_from_slice(&4u32.to_be_bytes());
                pack.extend_from_slice(&v.app_id.as_bytes());
                pack.extend_from_slice(&(v.migrate as u8).to_be_bytes());
                pack.extend_from_slice(&v.binary);
            }
            TransactionData::RawData(v) => {
                pack.extend_from_slice(&5u32.to_be_bytes());
                pack.extend_from_slice(v.as_slice())
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

impl From<Vec<SignedTransaction>> for TransactionList {
    fn from(value: Vec<SignedTransaction>) -> Self {
        let txs: Vec<_> = value.into_iter().map(Arc::new).collect();
        Self { txs }
    }
}

impl Into<Vec<SignedTransaction>> for TransactionList {
    fn into(self) -> Vec<SignedTransaction> {
        let txs: Vec<_> = self.txs.into_iter().map(|tx| tx.deref().clone()).collect();
        txs
    }
}

impl AsMut<Vec<Arc<SignedTransaction>>> for TransactionList {
    fn as_mut(&mut self) -> &mut Vec<Arc<SignedTransaction>> {
        &mut self.txs
    }
}

impl_codec_using_prost!(TransactionList);

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SignedTransaction {
    #[serde(flatten)]
    tx: Transaction,
    r: H256,
    s: H256,
    #[serde(with = "hex")]
    v: u32,
    //caches
    #[serde(skip)]
    hash: Arc<RwLock<Option<H256>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<Address>>>,
    #[serde(skip)]
    to: Arc<RwLock<Option<Address>>>,
}

// Custom Message Implementation
impl prost::Message for SignedTransaction {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        prost::encoding::message::encode(1u32, &self.tx, buf);
        prost::encoding::message::encode(2u32, &self.r, buf);
        prost::encoding::message::encode(3u32, &self.s, buf);
        if self.v != 0u32 {
            prost::encoding::uint32::encode(4u32, &self.v, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> core::result::Result<(), DecodeError>
    where
        B: Buf,
    {
        const STRUCT_NAME: &'static str = stringify!(SignedTransaction);
        match tag {
            1u32 => {
                let value = &mut self.tx;
                prost::encoding::message::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(tx));
                    error
                })
            }
            2u32 => {
                let value = &mut self.r;
                prost::encoding::message::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(r));
                    error
                })
            }
            3u32 => {
                let value = &mut self.s;
                prost::encoding::message::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(s));
                    error
                })
            }
            4u32 => {
                let value = &mut self.v;
                prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(v));
                    error
                })
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        0 + prost::encoding::message::encoded_len(1u32, &self.tx)
            + prost::encoding::message::encoded_len(2u32, &self.r)
            + prost::encoding::message::encoded_len(3u32, &self.s)
            + prost::encoding::uint32::encoded_len(4u32, &self.v)
    }
    fn clear(&mut self) {
        self.tx.clear();
        self.r.clear();
        self.s.clear();
        self.v = 0u32;
    }
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
        state.write(self.hash().as_bytes())
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, tx: Transaction) -> Result<Self> {
        let (r, s, v) = signature.rsv();

        Ok(Self {
            tx,
            r,
            s,
            v: v as u32,
            hash: Default::default(),
            from: Default::default(),
            to: Default::default(),
        })
    }

    pub fn hash(&self) -> H256 {
        cache(&self.hash, || Ok(self.tx.sig_hash()))
    }

    pub fn signature(&self) -> [u8; 65] {
        let sig = Signature::from_rsv((self.r, self.s, self.v as u8)).unwrap();
        sig.to_bytes()
    }

    pub fn nonce(&self) -> u64 {
        self.tx.nonce
    }
    pub fn sender(&self) -> Address {
        self.from()
    }

    pub fn to(&self) -> Address {
        let Some(data) = self.tx.data.as_ref() else  {
            return Address::default()
        };

        cache(&self.to, || match data {
            TransactionData::Payment(PaymentTx { to, .. }) => Ok(*to),
            TransactionData::Call(ApplicationCallTx { app_id, .. }) => Ok(*app_id),
            TransactionData::RawData(_) => Ok(Default::default()),
            TransactionData::Create(CreateApplicationTx { package_name, .. }) => {
                get_address_from_package_name(
                    &package_name,
                    self.from()
                        .network()
                        .ok_or_else(|| anyhow!("network not specified on senders address"))?,
                )
            }
            TransactionData::Update(UpdateApplicationTx { app_id, .. }) => Ok(*app_id),
        })
    }

    pub fn origin(&self) -> Address {
        self.from()
    }

    pub fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn data(&self) -> &TransactionData {
        self.tx.data.as_ref().unwrap()
    }

    pub fn raw_origin(&self) -> Result<PublicKey> {
        let signature = Signature::from_rsv((&self.r, &self.s, self.v as u8))?;
        let pub_key = signature.recover_public_key(&self.sig_hash()?)?;
        Ok(pub_key)
    }

    pub fn from(&self) -> Address {
        cache(&self.from, || {
            let signature = Signature::from_rsv((self.r, self.s, self.v as u8))?;
            signature
                .recover_public_key(self.sig_hash()?.as_slice())
                .map_err(|e| anyhow::anyhow!(e))
                .map(|key| get_address_from_pub_key(key, Network::from_chain_id(self.tx.chain_id)))
        })
    }
    pub fn network(&self) -> Network {
        self.from().network().unwrap_or_default()
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

impl Encodable for SignedTransaction {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(self.encode_to_vec())
    }
}

impl Decodable for SignedTransaction {
    fn decode(buf: &[u8]) -> Result<Self> {
        Message::decode(buf).map_err(|e| e.into())
    }
}
