use crate::BigArray;
use crate::{BlockHash, PubKey, Sig};
use anyhow::Result;
use codec::impl_codec;
use codec::{Decoder, Encoder};
use crypto::{RIPEMD160, SHA256};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransactionKind {
    Transfer {
        from: PubKey,
        to: PubKey,
        amount: u128,
        fee: u128,
    },
    Coinbase {
        miner: PubKey,
        amount: u128,
        block_hash: BlockHash,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    #[serde(with = "BigArray")]
    sig: Sig,
    origin: PubKey,
    nonce: u64,
    kind: TransactionKind,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash().eq(&other.hash())
    }
}

impl Eq for Transaction {}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.nonce.cmp(&other.nonce))
    }
}

impl Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.hash())
    }
}

impl Transaction {
    pub fn new(origin: PubKey, nonce: u64, sig: Sig, kind: TransactionKind) -> Self {
        Self {
            sig,
            origin,
            nonce,
            kind,
        }
    }

    pub fn origin(&self) -> &PubKey {
        &self.origin
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.signature());
        sha3.update(self.origin());
        sha3.update(&self.nonce().to_be_bytes());
        sha3.update(&self.kind.encode().unwrap());
        sha3.finalize(&mut out);
        out
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
    pub fn sender_address(&self) -> H160 {
        RIPEMD160::digest(&SHA256::digest(&self.origin))
    }
    pub fn fees(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, .. } => *fee,
            TransactionKind::Coinbase { .. } => 0,
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
}

impl_codec!(Transaction);
impl_codec!(TransactionKind);
