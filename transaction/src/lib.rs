use types::{AccountId, Sig, BlockHash};
use tiny_keccak::Hasher;
use account::{Account, TREASURY_ACCOUNT_PK};
use anyhow::Result;
use codec::impl_codec;
use codec::{Encoder, Decoder};
use serde::{Serialize, Deserialize};
use types::BigArray;

#[derive(Serialize, Deserialize, Clone)]
pub enum TransactionKind {
    Transfer { from: AccountId, to: AccountId, amount: u128},
    Coinbase { miner: AccountId, amount: u128, block_hash: BlockHash },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(with = "BigArray")]
    sig: Sig,
    origin: AccountId,
    nonce: u32,
    #[serde(flatten)]
    kind: TransactionKind,
}

impl Transaction {
    fn new(origin: AccountId, nonce: u32, sig: Sig, kind: TransactionKind) -> Self {
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

    pub fn id(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        match self.encode() {
            Ok(encoded_self) => {
                sha3.update(&encoded_self);
            }
            Err(_) => {}
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

    pub fn sig_hash(&self) -> Result<[u8;32]> {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.origin());
        sha3.update(&self.nonce());
        sha3.update(&self.kind.encode()?);
        sha3.finalize(&mut out);
       Ok(out)
    }
}

pub fn make_sign_transaction(account: &Account, nonce: u32, kind: TransactionKind) -> Result<Transaction> {
    let mut out = [0_u8; 32];
    let mut sha3 = tiny_keccak::Sha3::v256();
    sha3.update(&account.pub_key);
    sha3.update(&nonce.to_be_bytes());
    sha3.update(&kind.encode()?);
    sha3.finalize(&mut out);

    let sig = account.sign(&out)?;
    Ok(Transaction::new(account.pub_key.clone(), nonce, sig, kind))
}

pub fn verify_signed_transaction(transaction: &Transaction) -> Result<()> {
    account::verify_signature(transaction.origin(), transaction.signature(), &transaction.sig_hash()?)
}

pub fn verify_transaction_origin(origin: &[u8; 32], transaction: &Transaction) -> Result<()> {
    account::verify_signature(origin, transaction.signature(), &transaction.sig_hash()?)
}

impl_codec!(Transaction);
impl_codec!(TransactionKind);