use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use tiny_keccak::Hasher;

use account::GOVERNANCE_ACCOUNTID;
use codec::Encoder;
use crypto::{RIPEMD160, SHA256};
use crypto::ecdsa::SecretKey;
use primitive_types::{H160, H256};
use proto::UnsignedTransaction;
use types::account::{Account, get_address_from_pub_key};
use types::Address;
use types::tx::{SignedTransaction, Transaction};
use proto::Message;

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
    println!("Sig Hash On Signing {:?}", payload);
    let secrete = SecretKey::from_bytes(secret.as_fixed_bytes())?;
    println!("Signed Tx Key {:?}", H256::from(secrete.to_bytes()));
    let sig = secrete.sign(payload.as_bytes()).map_err(|e| anyhow!("{:?}", e))?;
    let tx = SignedTransaction::new(sig, tx)?;
    println!("Signed Tx By {:?}", tx.from());
    println!("Signed Tx By From Secrete {:?}", get_address_from_pub_key(secrete.public()));
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
