use std::cmp::Ordering;
use std::collections::BTreeSet;

use anyhow::{anyhow, Result};

use crypto::ecdsa::SecretKey;
use primitive_types::H256;
use types::account::{Account, Address42};
use types::network::Network;
use types::prelude::TransactionData;
use types::tx::{PaymentTx, SignedTransaction, Transaction, TransactionBuilder};

pub fn make_sign_transaction(
    account: &Account,
    nonce: u64,
    to: Address42,
    amount: u64,
    fee: u64,
) -> Result<SignedTransaction> {
    TransactionBuilder::with_signer(account)?
        .nonce(nonce)
        .fee(fee)
        .transfer()
        .to(to)
        .amount(amount)
        .build()
}

pub fn make_payment_sign_transaction(
    signer: H256,
    to: Address42,
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
        data: TransactionData::Payment(PaymentTx { to, amount }),
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
pub struct NoncePricedTransaction(pub SignedTransaction);

impl Eq for NoncePricedTransaction {}

impl PartialEq for NoncePricedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for NoncePricedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NoncePricedTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.nonce().cmp(&other.0.nonce()) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.0.price().cmp(&other.0.price()).reverse(),
            Ordering::Greater => Ordering::Greater,
        }
    }
}
pub type TransactionsByNonceAndPrice = BTreeSet<NoncePricedTransaction>;
