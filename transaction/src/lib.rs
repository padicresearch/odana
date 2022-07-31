use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::rc::Rc;

use anyhow::Result;
use tiny_keccak::Hasher;

use account::GOVERNANCE_ACCOUNTID;
use codec::Encoder;
use crypto::{RIPEMD160, SHA256};
use primitive_types::H160;
use types::account::Account;
use types::Address;
use types::tx::{SignedTransaction, Transaction};

pub fn make_sign_transaction(
    account: &Account,
    nonce: u64,
    to: Address,
    amount: u128,
    fee: u128,
) -> Result<SignedTransaction> {
    let data = Transaction {
        nonce,
        to,
        amount,
        fee,
    };
    let sig = account.sign(SHA256::digest(data.encode()).as_fixed_bytes())?;
    Ok(SignedTransaction::new(sig, data))
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

    #[test]
    fn generate_sudo_address() {
        let sudo_account = create_account();
        println!("{:?}", sudo_account);
    }
}
