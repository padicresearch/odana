use std::cmp::Ordering;
use std::fmt::Formatter;
use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use codec::impl_codec;
use codec::{Decoder, Encoder};
use crypto::ecdsa::{PublicKey, Signature};
use crypto::{RIPEMD160, SHA256};
use primitive_types::{H160, H256, H512, U128, U256, U512};

use crate::account::get_address_from_pub_key;
use crate::{cache_hash, Address, BigArray, Hash};

#[derive(Serialize, Deserialize)]
pub struct TransactionData {
    pub nonce: u64,
    pub kind: TransactionKind,
}

impl Encoder for TransactionData {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(105);
        buf.extend_from_slice(&self.nonce.to_be_bytes());
        buf.extend_from_slice(&self.kind.to_compact());
        Ok(buf)
    }
    fn encoded_size(&self) -> Result<u64> {
        unimplemented!()
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
pub enum TransactionKind {
    Transfer {
        from: Address,
        to: Address,
        amount: u128,
        fee: u128,
    },
}

pub const TRANSFER_KIND: u8 = 0x1;

impl TransactionKind {
    pub fn to_compact(&self) -> Vec<u8> {
        match &self {
            TransactionKind::Transfer {
                from,
                to,
                amount,
                fee,
            } => {
                let mut bytes = Vec::with_capacity(73);
                bytes.push(TRANSFER_KIND);
                bytes.extend_from_slice(&*from);
                bytes.extend_from_slice(&*to);
                bytes.extend_from_slice(&amount.to_be_bytes());
                bytes.extend_from_slice(&fee.to_be_bytes());
                bytes
            }
        }
    }
    pub fn from_compact(compact: &[u8]) -> Self {
        todo!()
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
                .field("from", &H160::from(from))
                .field("to", &H160::from(to))
                .field("amount", &amount)
                .field("fee", fee)
                .finish(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    nonce: u64,
    kind: TransactionKind,
    r: [u8; 32],
    s: [u8; 32],
    v: u8,
    //caches
    #[serde(skip)]
    hash: Arc<RwLock<Option<Hash>>>,
    #[serde(skip)]
    from: Arc<RwLock<Option<Address>>>,
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("r", &self.r)
            .field("s", &self.s)
            .field("v", &self.v)
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

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.hash().partial_cmp(&other.hash())
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash().cmp(&other.hash())
    }
}

impl std::hash::Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.hash())
    }
}

impl Transaction {
    pub fn new(nonce: u64, signature: Signature, kind: TransactionKind) -> Self {
        let (r, s, v) = signature.rsv();
        Self {
            nonce,
            kind,
            r,
            s,
            v,
            hash: Default::default(),
            from: Default::default(),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let hash = cache_hash(&self.hash, || {
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
    pub fn kind(&self) -> &TransactionKind {
        &self.kind
    }
    pub fn nonce(&self) -> u64 {
        self.nonce
    }
    pub fn sender(&self) -> H160 {
        self.origin()
    }

    pub fn to(&self) -> H160 {
        let to = match self.kind {
            TransactionKind::Transfer { to, .. } => to,
        };
        H160::from(to)
    }

    pub fn origin(&self) -> H160 {
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
    }

    pub fn raw_origin(&self) -> Result<PublicKey> {
        let signature = Signature::from_rsv((&self.r, &self.s, &self.v))?;
        let pub_key = signature.recover_public_key(&self.sig_hash()?)?;
        Ok(pub_key)
    }

    pub fn from(&self) -> H160 {
        let from = match self.kind {
            TransactionKind::Transfer { from, .. } => from,
        };
        H160::from(from)
    }

    pub fn fees(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, .. } => *fee,
        }
    }

    pub fn price(&self) -> u128 {
        match &self.kind {
            TransactionKind::Transfer { fee, amount, .. } => *fee + *amount,
        }
    }

    pub fn sig_hash(&self) -> Result<[u8; 32]> {
        let mut out = SHA256::digest(
            TransactionData {
                nonce: self.nonce,
                kind: self.kind.clone(),
            }
            .encode()?,
        );
        Ok(out.to_fixed_bytes())
    }

    pub fn size(&self) -> u64 {
        self.encoded_size().unwrap_or_default()
    }
}

impl_codec!(Transaction);
impl_codec!(TransactionKind);

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use primitive_types::{Compact, H160, U128};

    #[test]
    fn test_message_pack_serialization() {
        #[derive(Serialize, Deserialize, Clone)]
        pub struct Transfer {
            from: H160,
            to: H160,
            amount: U128,
            fee: U128,
            diff: U128,
        }

        #[derive(Serialize, Deserialize, Clone)]
        pub struct TransferAlt {
            from: [u8; 20],
            to: [u8; 20],
            amount: u128,
            fee: u128,
            diff: u128,
        }

        let tx = Transfer {
            from: Default::default(),
            to: Default::default(),
            amount: 1233.into(),
            fee: 123.into(),
            diff: 10.into(),
        };

        let tx_alt = TransferAlt {
            from: [1; 20],
            to: [2; 20],
            amount: 1233,
            fee: 123,
            diff: 10,
        };
        println!("{}", bincode::serialized_size(&tx).unwrap());
        println!("{}", bincode::serialized_size(&tx_alt).unwrap());
    }
}
