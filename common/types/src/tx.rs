use std::cmp::Ordering;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anyhow::Result;
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
use proto::tx::UnsignedTransaction;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Transaction {
    pub nonce: u64,
    pub to: Address,
    pub amount: u128,
    pub fee: u128,
}

impl Into<UnsignedTransaction> for Transaction {
    fn into(self) -> UnsignedTransaction {
        UnsignedTransaction {
            nonce: self.nonce,
            to: H160::from_slice(&self.to).to_string(),
            amount: self.amount.to_string(),
            fee: self.fee.to_string(),
        }
    }
}

impl Transaction {
    pub fn encode(self) -> Vec<u8> {
        let unsigned_tx: UnsignedTransaction = self.into();
        return unsigned_tx.encode_to_vec();
    }
    pub fn decode(buf: &[u8]) -> Result<Self> {
        let unsigned_tx: UnsignedTransaction = UnsignedTransaction::decode(buf)?;
        Ok(Self {
            nonce: unsigned_tx.nonce,
            to: H160::from_str(&unsigned_tx.to)?.to_fixed_bytes(),
            amount: unsigned_tx.amount.parse()?,
            fee: unsigned_tx.amount.parse()?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Confirmed,
    Pending,
    Queued,
    NotFound,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SignedTransaction {
    nonce: u64,
    to: Address,
    amount: u128,
    fee: u128,
    data: Vec<u8>,
    r: [u8; 32],
    s: [u8; 32],
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
            .field("to", &H160::from(&self.to))
            .field("amount", &self.amount)
            .field("fee", &self.fee)
            .field("data", &hex::encode(&self.data))
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
    pub fn new(signature: Signature, tx: Transaction) -> Self {
        let (r, s, v) = signature.rsv();
        Self {
            nonce: tx.nonce,
            to: tx.to,
            amount: tx.amount,
            fee: tx.fee,
            data: Default::default(),
            r,
            s,
            v,
            hash: Arc::new(Default::default()),
            from: Arc::new(Default::default()),
        }
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
        let sig = Signature::from_rsv((&self.r, &self.s, &self.v)).unwrap();
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
        let signature = Signature::from_rsv((&self.r, &self.s, &self.v))?;
        let pub_key = signature.recover_public_key(&self.sig_hash()?)?;
        Ok(pub_key)
    }

    pub fn from(&self) -> H160 {
        let origin = cache(&self.from, || {
            Signature::from_rsv((&self.r, &self.s, &self.v))
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
        self.fee
    }

    pub fn price(&self) -> u128 {
        self.amount
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let mut out = SHA256::digest(
            Transaction {
                nonce: self.nonce,
                to: self.to,
                amount: self.amount.into(),
                fee: self.fee.into(),
            }
                .encode(),
        );
        Ok(out.to_fixed_bytes())
    }

    pub fn size(&self) -> u64 {
        self.encoded_size().unwrap_or_default()
    }
}

impl_codec!(SignedTransaction);