use crate::consensus::check_transaction_fee;
use ed25519_dalek::{PublicKey, Verifier, Signature};
use anyhow::Result;
use tiny_keccak::Hasher;
use std::collections::HashMap;
use std::rc::Rc;
use crate::errors::BlockChainError;
use serde::{Serialize, Deserialize};
use storage::codec::{Encoder, Decoder};
use types::BigArray;
use crate::utxo::{UTXO, UTXOStore};
use crate::amount::TUCI;
use account::Account;
use codec::{Encoder, Decoder};

const MINER_REWARD: u128 = 10 * TUCI;

pub trait SerialHash {
    fn s_hash(&self) -> [u8; 32];
}


#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TxIn {
    pub pub_key: [u8; 32],
    pub prev_tx_id: [u8; 32],
    pub prev_tx_index: u16,
}

impl TxIn {
    pub fn new(prev_tx_id: [u8; 32], prev_tx_index: u16, pub_key: [u8; 32]) -> Self {
        Self {
            pub_key,
            prev_tx_id,
            prev_tx_index,
        }
    }
}

impl SerialHash for TxIn {
    fn s_hash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.pub_key);
        sha3.update(&self.prev_tx_id);
        sha3.update(&self.prev_tx_index.to_be_bytes());
        sha3.finalize(&mut out);
        out
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TxOut {
    pub pub_key: [u8; 32],
    pub value: u128,
}


impl TxOut {
    pub fn new(pub_key: [u8; 32], value: u128) -> Self {
        Self {
            pub_key,
            value,
        }
    }
}

impl SerialHash for TxOut {
    fn s_hash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.pub_key);
        sha3.update(&self.value.to_be_bytes());
        sha3.finalize(&mut out);
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub tx_id: [u8; 32],
    pub nonce: u128,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    #[serde(with = "BigArray")]
    pub sig: [u8; 64],
}

impl Tx {
    pub fn signed(account: &Account, nonce: u128, inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Result<Self> {
        if inputs.is_empty() && outputs.is_empty() {
            return Err(BlockChainError::TxInputOrOutputEmpty.into());
        }

        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&nonce.to_be_bytes());
        for tx in inputs.iter() {
            sha3.update(&tx.s_hash());
        }
        for tx in outputs.iter() {
            sha3.update(&tx.s_hash());
        }
        sha3.finalize(&mut out);

        let signature = account.sign(&out)?;


        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&out);
        sha3.finalize(&mut out);
        Ok(Self {
            tx_id: out,
            nonce,
            inputs,
            outputs,
            sig: signature,
        })
    }

    pub fn coinbase(account: &Account, tx_fees : u128) -> Result<Self> {
        let nonce: u128 = rand::random();

        let inputs = vec![TxIn::new([0; 32], 0, account.pub_key)];
        let outputs = vec![TxOut::new(account.pub_key, MINER_REWARD + tx_fees)];

        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&nonce.to_be_bytes());
        for tx in inputs.iter() {
            sha3.update(&tx.s_hash());
        }
        for tx in outputs.iter() {
            sha3.update(&tx.s_hash());
        }
        sha3.finalize(&mut out);

        let signature = account.sign(&out)?;


        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&out);
        sha3.finalize(&mut out);
        Ok(Self {
            tx_id: out,
            nonce,
            inputs,
            outputs,
            sig: signature,
        })
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.outputs.len() == 1
    }

    pub fn id(&self) -> &[u8; 32] {
        &self.tx_id
    }

    pub fn unsigned(nonce: u128, inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Result<Self> {
        if inputs.is_empty() && outputs.is_empty() {
            return Err(BlockChainError::TxInputOrOutputEmpty.into());
        }
        let mut inputs = inputs;
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        for tx in inputs.iter() {
            sha3.update(&tx.s_hash());
        }
        for tx in outputs.iter() {
            sha3.update(&tx.s_hash());
        }
        sha3.finalize(&mut out);
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&out);
        sha3.finalize(&mut out);
        Ok(Self {
            tx_id: out,
            nonce,
            inputs,
            outputs,
            sig: [0_u8; 64],
        })
    }

    pub fn sighash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.nonce.to_be_bytes());
        for tx in self.inputs.iter() {
            sha3.update(&tx.s_hash());
        }

        for tx in self.outputs.iter() {
            sha3.update(&tx.s_hash());
        }

        sha3.finalize(&mut out);
        out
    }

}

impl Encoder for Tx {}

impl Decoder for Tx {}

pub fn calculate_tx_in_out_amount(tx : &Tx, utxo : &UTXO) -> Result<(u128,u128)> {
    if tx.is_coinbase() {
        return Ok((MINER_REWARD, MINER_REWARD))
    }
    let mut input_amount: u128 = 0;
    for tx_in in tx.inputs.iter() {
        let coin = utxo.get_coin(tx_in.prev_tx_index as u16, &tx_in.prev_tx_id)?.ok_or(BlockChainError::VerifyError)?;
        input_amount += coin.tx_out.value
    }
    let out_amount = tx.outputs.iter().map(|out| out.value).fold(0, |acc, next| acc + next);

    Ok((input_amount, out_amount))

}


#[cfg(test)]
 mod tests {
    use storage::memstore::MemStore;
use std::collections::{HashMap, BTreeMap};
    use std::rc::Rc;
    use super::{TxOut, TxIn, Tx};
    use crate::account::{create_account, Account};
    use anyhow::Result;
    use ed25519_dalek::{PublicKey, Signature, Verifier};
    use crate::errors::BlockChainError;
    use crate::utxo::{UTXO, UTXOStore, CoinOut, CoinKey};
    use storage::{KVStore, KVEntry, PersistentStorage};
    use storage::codec::{Codec, Encoder, Decoder};
    use std::sync::{Arc, RwLock};
    use crate::consensus::validate_transaction;
    use std::collections::btree_map::Iter;
    use std::marker::PhantomData;
    use crate::block_storage::BlockStorage;
    use crate::mempool::MemPool;
    use crate::blockchain::BlockChainState;
    use account::{Account, create_account};


    pub struct TempStorage {
        pub utxo: Arc<UTXO>,
    }

    pub fn setup_storage(accounts: &Vec<Account>, storage: Arc<PersistentStorage>) -> TempStorage {
        let coin_base = [0_u8; 32];

        let res: Result<Vec<_>> = accounts.iter().map(|account| {
            Tx::coinbase(account, 0)
        }).collect();

        let txs = res.unwrap();


        let temp = TempStorage {
            utxo: Arc::new(UTXO::new(storage))
        };


        for tx in txs.iter() {
            temp.utxo.put(tx).unwrap()
        }


        temp
    }


    fn execute_tx(tx: Tx, utxo: &UTXO) -> Result<bool> {
        validate_transaction(&tx, utxo)?;
        for t in tx.inputs.iter() {
            utxo.spend(t.prev_tx_index, &t.prev_tx_id);
        }
        utxo.put(&tx);
        Ok(true)
    }

    fn get_account_coins(acount: &Account, utxo: &UTXO) -> Vec<(CoinKey, CoinOut)>{
        let mut res = vec![];
        for (k, v) in utxo.iter().unwrap() {
            let key =  k.unwrap();
            let coin = v.unwrap();
            if coin.tx_out.pub_key == acount.pub_key &&  !coin.is_spent{
                res.push((key,coin))
            }
        }
        res
        //Ok(valid)
    }

    fn available_balance(acount: &Account, utxo: &UTXO) -> u128{
        get_account_coins(acount,utxo).iter().map(|(_,v)| v.tx_out.value).fold(0,|acc, next|{
            let sum = acc + next;
            sum
        })
        //Ok(valid)
    }


    #[test]
    fn test_valid_tx() {
        let memstore = Arc::new(MemStore::new(vec![BlockStorage::column(), UTXO::column(), MemPool::column(), BlockChainState::column()]));
        let bob = create_account();
        let alice = create_account();
        let dave = create_account();
        let storage = setup_storage(&vec![bob, alice], Arc::new(PersistentStorage::InMemory(memstore.clone())));

        println!("{:#?}", memstore);
        let alice_coins = get_account_coins(&alice, &storage.utxo);
        println!("Alice Coins Count  : {} , Balance : {}", alice_coins.len(), available_balance(&alice, &storage.utxo));
        // Alice send bob 5 coins

        let tx_1 = {
             let tx_in = TxIn::new(alice_coins[0].0.tx_hash, alice_coins[0].0.index, alice.pub_key);
             let tx_out_1 = TxOut::new(alice.pub_key, 5);
             let tx_out_2 = TxOut::new(bob.pub_key, 3);
             let tx_out_3 = TxOut::new(dave.pub_key, 2);
             let tx = Tx::signed(&alice, 10,vec![tx_in], vec![tx_out_1, tx_out_2, tx_out_3]).unwrap();
             let tx_id = tx.tx_id;
             assert_eq!(execute_tx(tx, &storage.utxo).unwrap(), true);
             tx_id
         };

         println!("Bob Balance: {}", available_balance(&bob, &storage.utxo));
         println!("Alice Balance: {}", available_balance(&alice, &storage.utxo));
         println!("Dave Balance: {}", available_balance(&dave, &storage.utxo));
         println!("---------------------------------------------------------------------------------------------");

        storage.utxo.print();

    /*     {
             let tx_in_1 = TxIn::new(tx_1, 1, bob.pub_key);
             let tx_in_2 = TxIn::new(coinbase_tx_id, 0, bob.pub_key);
             let tx_out_1 = TxOut::new(bob.pub_key, 1);
             let tx_out_2 = TxOut::new(dave.pub_key, 12);
             let tx = Tx::signed(&bob, 10,vec![tx_in_1, tx_in_2], vec![tx_out_1, tx_out_2]).unwrap();
             assert_eq!(execute_tx(tx, &mut storage.unspent).unwrap(), true);
         }


         println!("Bob Balance: {}", available_balance(&bob.pub_key, &storage.unspent));
         println!("Alice Balance: {}", available_balance(&alice.pub_key, &storage.unspent));
         println!("Dave Balance: {}", available_balance(&dave.pub_key, &storage.unspent));*/
    }
}
