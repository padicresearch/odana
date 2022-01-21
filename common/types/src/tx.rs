use std::cmp::Ordering;
use std::fmt::Formatter;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use codec::{Decoder, Encoder};
use codec::impl_codec;
use crypto::{RIPEMD160, SHA256};
use primitive_types::{H160, H256, H512, U256, U512};

use crate::{BigArray, TxHash, Address, Hash};
use crate::{BlockHash, PubKey, Sig};
use ed25519_dalek::{Signature, PublicKey};

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Confirmed,
    Pending,
    Queued,
    NotFound,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TransactionKind {
    Transfer {
        from: Address,
        to: Address,
        amount: u128,
        fee: u128,
    }
}

impl std::fmt::Debug for TransactionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionKind::Transfer {
                from,
                to,
                amount,
                fee,
            } => f
                .debug_struct("Transfer")
                .field("from", &H256::from(from))
                .field("to", &H256::from(to))
                .field("amount", &amount)
                .field("fee", fee)
                .finish()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(with = "BigArray")]
    sig: Sig,
    nonce: u64,
    kind: TransactionKind,
    //caches
    #[serde(skip)]
    hash: Arc<RwLock<Option<TxHash>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<H160>>>,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("sig", &H512::from(self.sig))
            .field("nonce", &self.nonce)
            .field("kind", &self.kind)
            .finish()
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash().eq(&other.hash())
    }
}

impl Eq for Transaction {}

impl std::hash::Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.hash())
    }
}

impl Transaction {
    pub fn new(nonce: u64, sig: Sig, kind: TransactionKind) -> Self {
        Self {
            sig,
            nonce,
            kind,
            hash: Default::default(),
            from: Default::default(),
        }
    }

    pub fn origin(&self) -> &Hash {
        let sig = Signature::new(self.sig);
        PublicKey::
            & self.origin
    }

    pub fn hash(&self) -> [u8; 32] {
        match self.hash.read() {
            Ok(mut hash) => match *hash {
                None => {}
                Some(hash) => return hash,
            },
            Err(_) => {}
        }

        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.signature());
        sha3.update(self.origin());
        sha3.update(&self.nonce().to_be_bytes());
        sha3.update(&self.kind.encode().unwrap());
        sha3.finalize(&mut out);

        match self.hash.write() {
            Ok(mut hash) => *hash = Some(out.clone()),
            Err(_) => {}
        }

        out
    }

    pub fn hash_256(&self) -> H256 {
        H256::from(self.hash())
    }

    pub fn signature(&self) -> &Sig {
        &self.sig
    }
    pub fn kind(&self) -> &TransactionKind {
        &self.kind
    }
    pub fn nonce(&self) -> u64 {
        self.nonce
    }
    pub fn sender(&self) -> H160 {
        match self.from.read() {
            Ok(mut address) => match *address {
                None => {}
                Some(address) => return address,
            },
            Err(_) => {}
        }
        let out = RIPEMD160::digest(&SHA256::digest(&self.origin));

        match self.from.write() {
            Ok(mut address) => *address = Some(out.clone()),
            Err(_) => {}
        }

        out
    }

    pub fn to(&self) -> H160 {
        let to = match self.kind {
            TransactionKind::Transfer { to, .. } => to,
        };
        RIPEMD160::digest(&SHA256::digest(to))
    }

    pub fn fees(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, .. } => *fee
        }
    }

    pub fn price(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, amount, .. } => *fee + *amount
        }
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.origin());
        sha3.update(&self.nonce().to_be_bytes());
        sha3.update(&self.kind.encode()?);
        sha3.finalize(&mut out);
        Ok(out)
    }

    pub fn size(&self) -> u64 {
        self.encoded_size().unwrap_or_default()
    }
}

impl_codec!(Transaction);
impl_codec!(TransactionKind);
