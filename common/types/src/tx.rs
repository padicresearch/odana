use std::cmp::Ordering;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use serde::{Deserialize, Serialize};

use crypto::ecdsa::{PublicKey, Signature};
use crypto::sha256;
use primitive_types::H256;

use crate::account::{get_address_from_pub_key, Account, Address42};
use crate::network::Network;
use crate::{cache, Hash};

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
    pub to: Address42,
    pub amount: u64,
}

impl Message for PaymentTx {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::bytes::encode(1, &self.to, buf);
        prost::encoding::uint64::encode(2, &self.amount, buf);
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
            2 => prost::encoding::uint64::merge(wire_type, &mut self.amount, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "amount");
                    error
                },
            ),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::bytes::encoded_len(1, &self.to)
            + prost::encoding::uint64::encoded_len(2, &self.amount)
    }

    fn clear(&mut self) {}
}

#[derive(Serialize, Deserialize, PartialEq, Eq, prost::Message, Clone)]
pub struct ApplicationCallTx {
    #[prost(uint32, tag = "1")]
    pub app_id: u32,
    #[prost(bytes, tag = "2")]
    pub args: Vec<u8>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, prost::Oneof)]
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

const STRUCT_NAME: &str = "Transaction";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
pub struct Transaction {
    pub nonce: u64,
    pub chain_id: u32,
    pub genesis_hash: H256,
    pub fee: u64,
    #[serde(flatten)]
    pub data: TransactionData,
}

pub struct TransactionBuilder<'a> {
    tx: Transaction,
    account: &'a Account,
}

impl<'a> TransactionBuilder<'a> {
    pub fn with_signer(account: &'a Account) -> Result<Self> {
        let tx = Transaction {
            nonce: 0,
            chain_id: account
                .address
                .network()
                .ok_or_else(|| anyhow::anyhow!("network not specified on signer"))?
                .chain_id(),
            genesis_hash: Default::default(),
            fee: 0,
            data: Default::default(),
        };
        Ok(Self { tx, account })
    }

    pub fn chain_id(&mut self, chain_id: u32) -> &mut Self {
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

    pub fn fee(&mut self, fee: u64) -> &mut Self {
        self.tx.fee = fee;
        self
    }

    pub fn call(&'a mut self) -> AppCallTransactionBuilder<'a> {
        self.tx.data = TransactionData::Call(ApplicationCallTx::default());
        AppCallTransactionBuilder { inner: self }
    }

    pub fn transfer(&'a mut self) -> TransferTransactionBuilder<'a> {
        self.tx.data = TransactionData::Payment(PaymentTx::default());
        TransferTransactionBuilder { inner: self }
    }

    pub fn build(&'a mut self) -> Result<SignedTransaction> {
        self.account
            .sign(self.tx.sig_hash().as_bytes())
            .and_then(|sig| SignedTransaction::new(sig, self.tx.clone()))
    }
}

pub struct TransferTransactionBuilder<'a> {
    inner: &'a mut TransactionBuilder<'a>,
}

impl<'a> TransferTransactionBuilder<'a> {
    pub fn to(&mut self, to: Address42) -> &mut Self {
        if let TransactionData::Payment(pmt) = &mut self.inner.tx.data {
            pmt.to = to
        }
        self
    }

    pub fn amount(&mut self, amount: u64) -> &mut Self {
        if let TransactionData::Payment(pmt) = &mut self.inner.tx.data {
            pmt.amount = amount
        }
        self
    }

    pub fn build(&mut self) -> Result<SignedTransaction> {
        self.inner
            .account
            .sign(self.inner.tx.sig_hash().as_bytes())
            .and_then(|sig| SignedTransaction::new(sig, self.inner.tx.clone()))
    }
}
pub struct AppCallTransactionBuilder<'a> {
    inner: &'a mut TransactionBuilder<'a>,
}

impl<'a> AppCallTransactionBuilder<'a> {
    pub fn app_id(&mut self, app_id: u32) -> &mut Self {
        if let TransactionData::Call(call) = &mut self.inner.tx.data {
            call.app_id = app_id
        }
        self
    }

    pub fn args(&mut self, args: Vec<u8>) -> &mut Self {
        if let TransactionData::Call(call) = &mut self.inner.tx.data {
            call.args = args
        }
        self
    }

    pub fn build(&mut self) -> Result<SignedTransaction> {
        self.inner
            .account
            .sign(self.inner.tx.sig_hash().as_bytes())
            .and_then(|sig| SignedTransaction::new(sig, self.inner.tx.clone()))
    }
}
pub struct RawDataTransactionBuilder<'a> {
    tx: Transaction,
    account: &'a Account,
}

impl<'a> RawDataTransactionBuilder<'a> {
    pub fn with_raw(&mut self, data: String) -> &mut Self {
        if let TransactionData::RawData(raw) = &mut self.tx.data {
            *raw = data
        }
        self
    }

    pub fn build(self) -> Result<SignedTransaction> {
        self.account
            .sign(self.tx.sig_hash().as_bytes())
            .and_then(|sig| SignedTransaction::new(sig, self.tx))
    }
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
            5 | 6 | 7 => {
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
        match &self.data {
            TransactionData::Payment(pmt) => {
                pack.extend_from_slice(&1u32.to_be_bytes());
                pack.extend_from_slice(&pmt.amount.to_be_bytes());
                pack.extend_from_slice(pmt.to.as_bytes());
            }
            TransactionData::Call(call) => {
                pack.extend_from_slice(&2u32.to_be_bytes());
                pack.extend_from_slice(&call.app_id.to_be_bytes());
                pack.extend_from_slice(&call.args);
            }
            TransactionData::RawData(raw) => {
                pack.extend_from_slice(&3u32.to_be_bytes());
                pack.extend_from_slice(raw.as_bytes())
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
    hash: Arc<RwLock<Option<H256>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<Address42>>>,
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
        Ok(Self {
            tx,
            r,
            s,
            v,
            hash: Arc::new(Default::default()),
            from: Arc::new(Default::default()),
        })
    }

    pub fn hash(&self) -> H256 {
        cache(&self.hash, || self.tx.sig_hash())
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
    pub fn sender(&self) -> Address42 {
        self.from()
    }

    pub fn to(&self) -> Address42 {
        match &self.tx.data {
            TransactionData::Payment(pmt) => pmt.to,
            TransactionData::Call(_) => Default::default(),
            TransactionData::RawData(_) => Default::default(),
        }
    }

    pub fn origin(&self) -> Address42 {
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

    pub fn from(&self) -> Address42 {
        cache(&self.from, || {
            Signature::from_rsv((&self.r, &self.s, self.v))
                .map_err(|e| anyhow::anyhow!(e))
                .and_then(|signature| {
                    self.sig_hash().and_then(|sig_hash| {
                        signature
                            .recover_public_key(&sig_hash)
                            .map_err(|e| anyhow::anyhow!(e))
                            .map(|key| {
                                get_address_from_pub_key(
                                    key,
                                    Network::from_chain_id(self.tx.chain_id),
                                )
                            })
                    })
                })
                .unwrap_or_default()
        })
    }

    pub fn fees(&self) -> u64 {
        self.tx.fee
    }

    pub fn price(&self) -> u64 {
        match &self.tx.data {
            TransactionData::Payment(p) => p.amount,
            TransactionData::Call(_) => 10_000_000,
            TransactionData::RawData(s) => (s.len() as u64) * 1000,
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
        prost::encoding::bytes::encode(8, &self.r, buf);
        prost::encoding::bytes::encode(9, &self.s, buf);
        prost::encoding::bytes::encode(10, &self.v, buf);
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
            8 => prost::encoding::bytes::merge(wire_type, &mut self.r, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "r");
                    error
                },
            ),
            9 => prost::encoding::bytes::merge(wire_type, &mut self.s, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "s");
                    error
                },
            ),
            10 => prost::encoding::bytes::merge(wire_type, &mut self.v, buf, ctx).map_err(
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
            + prost::encoding::bytes::encoded_len(8, &self.r)
            + prost::encoding::bytes::encoded_len(9, &self.s)
            + prost::encoding::bytes::encoded_len(10, &self.v)
    }

    fn clear(&mut self) {}
}

#[cfg(test)]
mod tests {
    use crate::account::{get_address_from_pub_key, Account};
    use crate::network::Network;
    use crate::prelude::{SignedTransaction, TransactionList};
    use crate::tx::{AnyType, TransactionBuilder};
    use crypto::ecdsa::Keypair;
    use hex::ToHex;
    use primitive_types::{H160, H256};
    use prost::Message;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;
    use std::sync::Arc;

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
            .to(user2.address)
            .amount(1_000_000)
            .build()
            .unwrap();
        println!("{}", hex::encode(tx.encode_to_vec(), false));
        txs.as_mut().push(Arc::new(tx));
        let tx = TransactionBuilder::with_signer(&user1)
            .unwrap()
            .nonce(1)
            .fee(100_000)
            .genesis_hash(genesis)
            .call()
            .app_id(20)
            .args(prost::Message::encode_to_vec(&TransferToken {
                token_id: 23,
                from: "ama".to_string(),
                to: "kofi".to_string(),
                amount: "1000".to_string(),
            }))
            .build()
            .unwrap();
        txs.as_mut().push(Arc::new(tx));
        let tx = TransactionBuilder::with_signer(&user1)
            .unwrap()
            .nonce(1)
            .fee(100_000)
            .genesis_hash(genesis)
            .build()
            .unwrap();
        txs.as_mut().push(Arc::new(tx));

        let txs_ctl = TransactionList::decode(txs.encode_to_vec().as_slice()).unwrap();
        assert_eq!(txs, txs_ctl);
        println!("{}", serde_json::to_string_pretty(&txs).unwrap());
    }

    #[test]
    fn test_encoding_and_decoding_2() {
        let a = "0x0a0330783112033078311a4230783030303030303030303030303030303030303030303\
        03030303030303030303030303030303030303030303030303030303030303030303030303030303030302207\
        307831383661302a350a2a3078366266383336363338653164653637343133626331353732623463623161656\
        13834623038333663120730786634323430424230783962313932323737316435303435323263333638376539\
        33383432323733363330383665303333623061306434666430353137333730323063653137326363354a42307\
        83435386132383630626437393234326239363466383838643934633666663261643865396632656261613736\
        62623465353236663333616665646562306663635203307830";
        let tx = SignedTransaction::decode(hex::decode(a).unwrap().as_slice()).unwrap();
        let b = hex::encode(tx.encode_to_vec(), false);
        assert_eq!(a, b)
    }
}
