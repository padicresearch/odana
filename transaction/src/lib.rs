use std::cmp::Ordering;
use std::collections::BTreeSet;

use anyhow::{anyhow, Result};

use crypto::ecdsa::SecretKey;
use primitive_types::{Address, H256};

use types::network::Network;
use types::prelude::TransactionData;
use types::tx::{PaymentTx, SignedTransaction, Transaction};

pub fn make_payment_sign_transaction(
    signer: H256,
    to: Address,
    nonce: u64,
    amount: u64,
    fee: u64,
    network: Network,
) -> Result<SignedTransaction> {
    let chain_id = network.chain_id();
    let tx = Transaction {
        nonce,
        chain_id,
        genesis_hash: Default::default(),
        fee,
        value: amount,
        data: TransactionData::Payment(PaymentTx { to }),
    };
    sign_tx(signer, tx)
}

pub fn make_signed_transaction(
    signer: H256,
    nonce: u64,
    amount: u64,
    fee: u64,
    network: Network,
    data: TransactionData,
) -> Result<SignedTransaction> {
    let chain_id = network.chain_id();
    let tx = Transaction {
        nonce,
        chain_id,
        genesis_hash: Default::default(),
        fee,
        value: amount,
        data,
    };
    sign_tx(signer, tx)
}

pub fn sign_tx(secret: H256, tx: Transaction) -> Result<SignedTransaction> {
    let payload = tx.sig_hash();
    let secrete = SecretKey::from_bytes(secret.as_fixed_bytes())?;
    let sig = secrete
        .sign(payload.as_bytes())
        .map_err(|e| anyhow!("{:?}", e))?;
    let tx = SignedTransaction::new(sig, tx)?;
    Ok(tx)
}

#[derive(Debug)]
pub struct NoncePricedTransaction<'a>(pub &'a SignedTransaction);

impl<'a> Eq for NoncePricedTransaction<'a> {}

impl<'a> PartialEq for NoncePricedTransaction<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other.0)
    }
}

impl<'a> PartialOrd for NoncePricedTransaction<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for NoncePricedTransaction<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.nonce().cmp(&other.0.nonce()) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.0.price().cmp(&other.0.price()).reverse(),
            Ordering::Greater => Ordering::Greater,
        }
    }
}

pub type TransactionsByNonceAndPrice<'a> = BTreeSet<NoncePricedTransaction<'a>>;
