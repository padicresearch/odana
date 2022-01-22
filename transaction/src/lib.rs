use account::GOVERNANCE_ACCOUNTID;
use anyhow::Result;
use codec::Encoder;
use crypto::{RIPEMD160, SHA256};
use primitive_types::H160;
use tiny_keccak::Hasher;
use types::account::Account;
use types::tx::{Transaction, TransactionKind, TransactionData};

pub fn make_sign_transaction(
    account: &Account,
    nonce: u64,
    kind: TransactionKind,
) -> Result<Transaction> {
    let data = TransactionData {
        nonce,
        kind: kind.clone(),
    };
    let sig = account.sign(SHA256::digest(data.encode()?).as_fixed_bytes())?;
    Ok(Transaction::new(nonce, sig.to_bytes(), kind))
}

#[cfg(test)]
mod test {
    use account::create_account;

    #[test]
    fn generate_sudo_address() {
        let sudo_account = create_account();
        println!("{:?}", sudo_account);
    }
}
