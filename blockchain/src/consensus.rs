use crate::blockchain::{ BLOCK_DIFFICULTY};
use crate::errors::BlockChainError;
use crate::transaction::Tx;
use crate::utxo::{UTXOStore, UTXO};
use anyhow::Result;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use types::BlockHash;
use types::block::Block;
use merkle::Merkle;

pub fn validate_transaction(tx: &Tx, utxo: &UTXO) -> Result<()> {
    if tx.is_coinbase() {
        return Ok(())
    }
    let sighash = tx.sighash();
    let mut input_amount: u128 = 0;
    let sig = Signature::new(tx.sig);
    for tx_in in tx.inputs.iter() {
        let coin = utxo
            .get_coin(tx_in.prev_tx_index as u16, &tx_in.prev_tx_id)?
            .ok_or(BlockChainError::VerifyError)?;
        let prev_tx_out = if coin.is_spent {
            return Err(BlockChainError::InvalidTransactionCoinSpent.into());
        } else {
            coin.tx_out
        };
        let pub_key = PublicKey::from_bytes(&tx_in.pub_key)?;
        if !(tx_in.pub_key == prev_tx_out.pub_key && pub_key.verify(&sighash, &sig).is_ok()) {
            return Err(BlockChainError::InvalidTransaction.into());
        }
        input_amount += prev_tx_out.value
    }
    let out_amount = tx
        .outputs
        .iter()
        .map(|out| out.value)
        .fold(0, |acc, next| acc + next);
    if input_amount >= out_amount {Ok(())} else {Err(BlockChainError::InvalidTransaction.into())}
}

pub fn validate_chain(block_height: u128, block_storage: ()) -> Result<()> {
    Ok(())
}

pub fn check_transaction_fee(in_amount: u128, out_amount: u128) -> Result<u128> {
    // Check 1% fee on transaction
    let min_fee = ((in_amount as f64) * 0.01) as u128;

    let fee = in_amount.saturating_sub(out_amount);

    if fee < min_fee {
        return Err(BlockChainError::TransactionFeeTooLow.into());
    }

    Ok(fee)
}


pub fn check_block_pow(block_hash: &BlockHash) -> bool {
    let block_hash_encoded = hex::encode(block_hash);
    block_hash_encoded.starts_with(BLOCK_DIFFICULTY)
}

pub fn validate_block(block: &Block) -> Result<()> {
    let block_hash = block.calculate_hash();
    if !check_block_pow(&block_hash) {
        return Err(BlockChainError::InvalidBlock.into())
    }
    let mut merkle = Merkle::default();
    for tx in block.transactions().iter() {
        let _ = merkle.update(&tx[..])?;
    }
    let merkle_root = merkle.finalize().ok_or(BlockChainError::MerkleError)?;
    if merkle_root != block.merkle_root() {
        return Err(BlockChainError::InvalidBlock.into())
    }
    Ok(())
}



pub fn execute_tx(tx: Tx, utxo: &UTXO) -> Result<()> {
    validate_transaction(&tx, utxo)?;
    if !tx.is_coinbase() {
        for t in tx.inputs.iter() {
            utxo.spend(t.prev_tx_index, &t.prev_tx_id)?;
        }
    }
    utxo.put(&tx)?;
    Ok(())
}