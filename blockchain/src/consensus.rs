use crate::utxo::{UTXO, UTXOStore};
use crate::transaction::Tx;
use ed25519_dalek::{PublicKey, Verifier, Signature};
use crate::errors::BlockChainError;

pub fn validate_transaction(tx : &Tx, utxo: &UTXO) -> Result<bool, BlockChainError> {
    let sighash = tx.sighash();
    let mut input_amount: u128 = 0;
    let sig = Signature::new(tx.sig);
    for tx_in in tx.inputs.iter() {
        let coin = utxo.get_coin(tx_in.prev_tx_index as u16, &tx_in.prev_tx_id)?.ok_or(BlockChainError::VerifyError)?;
        let prev_tx_out = if coin.is_spent {
            return Ok(false)
        }else {
            coin.tx_out
        };
        let pub_key = PublicKey::from_bytes(&tx_in.pub_key)?;
        if !(tx_in.pub_key == prev_tx_out.pub_key && pub_key.verify(&sighash, &sig).is_ok()) {
            return Ok(false);
        }
        input_amount += prev_tx_out.value
    }
    let out_amount = tx.outputs.iter().fold(0, |acc, tx| tx.value);
    Ok(input_amount >= out_amount)
}