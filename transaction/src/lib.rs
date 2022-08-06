use std::cmp::Ordering;
use std::collections::BTreeSet;
use anyhow::{anyhow, Result};
use codec::Encoder;
use crypto::ecdsa::SecretKey;
use crypto::SHA256;
use primitive_types::H256;
use proto::UnsignedTransaction;
use types::account::Account;
use types::Address;
use types::tx::{SignedTransaction, Transaction};


pub fn make_sign_transaction(
    account: &Account,
    nonce: u64,
    to: Address,
    amount: u128,
    fee: u128,
    data: String,
) -> Result<SignedTransaction> {
    let data = Transaction {
        nonce,
        to: to.into(),
        amount: amount.into(),
        fee: fee.into(),
        data,
    };
    let sig = account.sign(SHA256::digest(data.encode()?).as_fixed_bytes())?;
    SignedTransaction::new(sig, data.into_proto()?)
}

pub fn sign_tx(secret: H256, tx: UnsignedTransaction) -> Result<SignedTransaction> {
    let raw = Transaction::from_proto(&tx)?;
    let payload = raw.sig_hash();
    let secrete = SecretKey::from_bytes(secret.as_fixed_bytes())?;
    let sig = secrete.sign(payload.as_bytes()).map_err(|e| anyhow!("{:?}", e))?;
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

#[cfg(test)]
mod test {
    use account::create_account;
    use proto::TransactionStatus;

    #[test]
    fn generate_sudo_address() {
        let sudo_account = create_account();
        println!("{:?}", sudo_account);
    }
}
