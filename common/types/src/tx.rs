use anyhow::Result;
use codec::impl_codec;
use codec::{Decoder, Encoder};
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;
use crate::BigArray;
use crate::{AccountId, BlockHash, Sig};
use primitive_types::H160;
use crypto::{Ripe160, SHA256};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TransactionKind {
    Transfer {
        from: AccountId,
        to: AccountId,
        amount: u128,
        fee: u128,
    },
    Coinbase {
        miner: AccountId,
        amount: u128,
        block_hash: BlockHash,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    #[serde(with = "BigArray")]
    sig: Sig,
    origin: AccountId,
    nonce: u32,
    kind: TransactionKind,
}

impl Transaction {
    pub fn new(origin: AccountId, nonce: u32, sig: Sig, kind: TransactionKind) -> Self {
        Self {
            sig,
            origin,
            nonce,
            kind,
        }
    }

    pub fn origin(&self) -> &AccountId {
        &self.origin
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        match self.encode() {
            Ok(encoded_self) => {
                sha3.update(&encoded_self);
            }
            Err(e) => {
                panic!(e)
            }
        }
        sha3.finalize(&mut out);
        out
    }

    pub fn signature(&self) -> &Sig {
        &self.sig
    }
    pub fn kind(&self) -> &TransactionKind {
        &self.kind
    }
    pub fn nonce(&self) -> [u8; 4] {
        self.nonce.to_be_bytes()
    }
    pub fn nonce_u32(&self) -> u32 {
        self.nonce
    }
    pub fn sender_address(&self) -> H160 {
        Ripe160::digest(&SHA256::digest(&self.origin))
    }
    pub fn fees(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, .. } => {
                *fee
            }
            TransactionKind::Coinbase { .. } => {
                0
            }
        }
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.origin());
        sha3.update(&self.nonce());
        sha3.update(&self.kind.encode()?);
        sha3.finalize(&mut out);
        Ok(out)
    }
}

impl_codec!(Transaction);
impl_codec!(TransactionKind);