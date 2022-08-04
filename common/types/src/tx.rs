use std::cmp::Ordering;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use crate::account::get_address_from_pub_key;
use crate::{cache, Address, BigArray, Hash};
use codec::impl_codec;
use codec::{Decoder, Encoder};
use crypto::ecdsa::{PublicKey, Signature};
use crypto::{RIPEMD160, SHA256};
use primitive_types::{H160, H256, H512, U128, U256, U512};
use prost::Message;
use proto::{TransactionStatus, UnsignedTransaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(with = "crate::uint_hex_codec")]
    pub nonce: u64,
    pub to: H160,
    pub amount: U128,
    pub fee: U128,
    pub data: String,
}

impl Transaction {
    pub fn into_proto(self) -> Result<UnsignedTransaction> {
        let json_rep = serde_json::to_vec(&self)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn from_proto(msg: &UnsignedTransaction) -> Result<Transaction> {
        let json_rep = serde_json::to_vec(msg)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn sig_hash(&self) -> H256 {
        let mut pack = Vec::new();
        pack.extend_from_slice(&self.nonce.to_be_bytes());
        pack.extend_from_slice(&self.to.as_bytes());
        pack.extend_from_slice(&self.amount.to_be_bytes());
        pack.extend_from_slice(&self.fee.to_be_bytes());
        pack.extend_from_slice(&self.data.as_bytes());
        return SHA256::digest(pack)
    }
}

impl Encoder for Transaction {
    fn encode(&self) -> Result<Vec<u8>> {
        let unsigned_tx: Result<UnsignedTransaction> = self.clone().into_proto();
        unsigned_tx
            .map(|tx| tx.encode_to_vec())
            .map_err(|e| anyhow!("{}", e))
    }
}

impl Decoder for Transaction {
    fn decode(buf: &[u8]) -> Result<Self> {
        let unsigned_tx: UnsignedTransaction = UnsignedTransaction::decode(buf)?;
        let json_rep = serde_json::to_vec(&unsigned_tx)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionList {
    pub txs: Vec<Arc<SignedTransaction>>,
}

impl TransactionList {
    pub fn new(txs: Vec<Arc<SignedTransaction>>) -> Self {
        Self {
            txs
        }
    }
}

impl AsRef<Vec<Arc<SignedTransaction>>> for TransactionList {
    fn as_ref(&self) -> &Vec<Arc<SignedTransaction>> {
        &self.txs
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SignedTransaction {
    #[serde(with = "crate::uint_hex_codec")]
    nonce: u64,
    to: H160,
    amount: U128,
    fee: U128,
    data: String,
    r: H256,
    s: H256,
    #[serde(with = "crate::uint_hex_codec")]
    v: u8,
    //caches
    #[serde(skip)]
    hash: Arc<RwLock<Option<Hash>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<H160>>>,
}

impl std::fmt::Debug for SignedTransaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("nonce", &self.nonce)
            .field("to", &self.to)
            .field("amount", &self.amount)
            .field("fee", &self.fee)
            .field("data", &self.data)
            .field("r", &self.r)
            .field("s", &self.s)
            .field("v", &self.v)
            .finish()
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
        state.write(&self.hash())
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, tx: UnsignedTransaction) -> Result<Self> {
        let (r, s, v) = signature.rsv();
        Ok(Self {
            nonce: U128::from_str(&tx.nonce)?.as_u64(),
            to: H160::from_str(&tx.to)?,
            amount: U128::from_str(&tx.amount)?,
            fee: U128::from_str(&tx.fee)?,
            data: tx.data.to_owned(),
            r,
            s,
            v,
            hash: Arc::new(Default::default()),
            from: Arc::new(Default::default()),
        })
    }

    pub fn hash(&self) -> [u8; 32] {
        let hash = cache(&self.hash, || {
            SHA256::digest(self.encode().unwrap()).to_fixed_bytes()
        });
        hash
    }

    pub fn hash_256(&self) -> H256 {
        H256::from(self.hash())
    }

    pub fn signature(&self) -> [u8; 65] {
        let sig = Signature::from_rsv((self.r, self.s, self.v)).unwrap();
        sig.to_bytes()
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }
    pub fn sender(&self) -> H160 {
        self.from()
    }

    pub fn to(&self) -> H160 {
        H160::from(self.to)
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
        let origin = cache(&self.from, || {
            Signature::from_rsv((&self.r, &self.s, self.v))
                .map_err(|e| anyhow::anyhow!(e))
                .and_then(|signature| {
                    self.sig_hash().and_then(|sig_hash| {
                        signature
                            .recover_public_key(&sig_hash)
                            .map_err(|e| anyhow::anyhow!(e))
                            .and_then(|pub_key| Ok(get_address_from_pub_key(pub_key)))
                    })
                })
                .unwrap_or_default()
        });
        origin
    }

    pub fn fees(&self) -> u128 {
        self.fee.as_u128()
    }

    pub fn price(&self) -> u128 {
        self.amount.as_u128()
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let tx = Transaction {
            nonce: self.nonce,
            to: self.to,
            amount: self.amount.into(),
            fee: self.fee.into(),
            data: self.data.clone(),
        };
        let raw = tx.sig_hash();
        Ok(raw.to_fixed_bytes())
    }

    pub fn into_proto(self) -> Result<proto::Transaction> {
        let json_rep = serde_json::to_vec(&self)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn into_raw(self) -> Result<String> {
        let tx = self.into_proto()?;
        let encoded = tx.encode_to_vec();
        Ok(prefix_hex::encode(encoded))
    }

    pub fn from_proto(msg: proto::Transaction) -> Result<Self> {
        let json_rep = serde_json::to_vec(&msg)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn from_raw(buf: &[u8]) -> Result<Self> {
        let tx: proto::Transaction = proto::Transaction::decode(buf)?;
        let json_rep = serde_json::to_vec(&tx)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn from_raw_str(buf: &str) -> Result<Self> {
        let decoded: Vec<u8> = prefix_hex::decode(buf).map_err(|e| anyhow!("{}", e))?;
        let tx: proto::Transaction = proto::Transaction::decode(decoded.as_slice())?;
        let json_rep = serde_json::to_vec(&tx)?;
        serde_json::from_slice(&json_rep).map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn size(&self) -> u64 {
        self.encoded_size().unwrap_or_default()
    }
}

impl_codec!(SignedTransaction);

#[test]
fn test_proto_conversions() {
    let tx = Transaction {
        nonce: 1000,
        to: H160::from([1; 20]),
        amount: U128::from(1000000),
        fee: U128::from(200),
        data: "".to_string(),
    };

    let utx = tx.into_proto().unwrap();
    println!("{:#?}", utx);

    let tx2 = Transaction::decode(&utx.encode_to_vec()).unwrap();
    println!("{:#?}", tx2);
    //println!("{:#02x}", U128::from(5));
}


#[test]
fn test_tx_status() {
    let status = vec![TransactionStatus::Confirmed, TransactionStatus::Pending, TransactionStatus::NotFound];
    println!("{}", serde_json::to_string_pretty(&status).unwrap());
}