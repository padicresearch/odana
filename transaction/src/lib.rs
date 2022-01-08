use account::GOVERNANCE_ACCOUNTID;
use anyhow::Result;
use codec::Encoder;
use crypto::{RIPEMD160, SHA256};
use primitive_types::H160;
use tiny_keccak::Hasher;
use types::account::Account;
use types::tx::{Transaction, TransactionKind};
use types::{BlockHash, PubKey};

pub fn make_sign_transaction(
    account: &Account,
    nonce: u64,
    kind: TransactionKind,
) -> Result<Transaction> {
    let mut out = [0_u8; 32];
    let mut sha3 = tiny_keccak::Sha3::v256();
    sha3.update(&account.pub_key);
    sha3.update(&nonce.to_be_bytes());
    sha3.update(&kind.encode()?);
    sha3.finalize(&mut out);

    let sig = account.sign(&out)?;
    Ok(Transaction::new(account.pub_key.clone(), nonce, sig, kind))
}

pub fn validate_transaction(
    transaction: &Transaction,
    block: Option<BlockHash>,
    block_miner: Option<PubKey>,
) -> Result<()> {
    match transaction.kind() {
        TransactionKind::Transfer { from, .. } => {
            if from != transaction.origin() && transaction.origin() != &GOVERNANCE_ACCOUNTID {
                anyhow::bail!("Bad origin")
            }
        }
        TransactionKind::Coinbase {
            block_hash, miner, ..
        } => {
            if let (Some(block), Some(block_miner)) = (block, block_miner) {
                if block != *block_hash && block_miner != *miner {
                    anyhow::bail!("invalid coinbase transaction")
                }
            } else {
                anyhow::bail!("block and block miner args not provided")
            }
        }
    }
    account::verify_signature(
        transaction.origin(),
        transaction.signature(),
        &transaction.sig_hash()?,
    )
}

pub fn verify_transaction_origin(origin: &[u8; 32], transaction: &Transaction) -> Result<()> {
    account::verify_signature(origin, transaction.signature(), &transaction.sig_hash()?)
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
