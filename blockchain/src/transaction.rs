use ed25519_dalek::{PublicKey, Verifier, Signature};
use anyhow::Result;
use tiny_keccak::Hasher;
use std::collections::HashMap;
use std::rc::Rc;
use crate::errors::BlockChainError;
use crate::account::Account;

const MINER_REWARD : u128 = 0;

pub trait SerialHash {
    fn s_hash(&self) -> [u8;32];
}


#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Clone)]
pub struct Tx {
    pub tx_id: [u8; 32],
    pub nonce : u128,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub sig: [u8; 64]
}

impl Tx {
    pub fn signed(account: &Account, nonce : u128, inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Result<Self> {
        if inputs.is_empty() && outputs.is_empty() {
            return Err(BlockChainError::TxInputOrOutputEmpty.into())
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
            sig: signature
        })
    }

    pub fn coinbase(account: &Account) -> Result<Self> {

        let nonce = rand::random();

        let inputs = vec![TxIn::new([0;32],0, account.pub_key)];
        let outputs = vec![TxOut::new(account.pub_key,MINER_REWARD)];

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
            sig: signature
        })
    }

    pub fn is_coinbase(&self) -> bool {

        self.inputs.len() == 1 && self.outputs.len() == 1
    }

    pub fn id(&self) -> &[u8;32] {
        &self.tx_id
    }

    pub fn unsigned(nonce : u128,inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Result<Self> {
        if inputs.is_empty() && outputs.is_empty() {
            return Err(BlockChainError::TxInputOrOutputEmpty.into())
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
            sig: [0_u8;64]
        })
    }

    pub fn sighash(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&nonce.to_be_bytes());
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


#[cfg(test)]
mod tests {
    use std::collections::{HashMap, BTreeMap};
    use std::rc::Rc;
    use super::{TxOut, TxIn, Tx};
    use crate::account::{create_account, Account};
    use anyhow::Result;
    use ed25519_dalek::{PublicKey, Signature, Verifier};
    use crate::errors::BlockChainError;
    use crate::utxo::{UTXO, UTXOStore};
    use storage::{Storage, KVEntry};
    use storage::codec::{Codec, Encoder, Decoder};
    use std::sync::{Arc, RwLock};
    use crate::consensus::validate_transaction;

    struct MemStore {
        inner : Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>
    }

    impl MemStore {
        fn new() -> Self {
            Self {
                inner: Arc::new(Default::default())
            }
        }
    }

    impl<S : KVEntry> Storage<S > for MemStore{
        fn get(&self, key: &S::Key) -> Result<Option<S::Value>> {
            let store = self.inner.clone().read()?;
            let key = key.encode()?;
            let result = store.get(&key);
            match result {
                Some(value) =>  {
                   Ok(S::Value::decode(value)?)
                },
                None => Ok(None),
            }

        }

        fn put(&self, key: S::Key, value: S::Value) -> Result<()> {
            let mut store = self.inner.clone().write()?;
            let key =key.encode()?;
            let value = value.encode()?;
            store.insert(key,value);
            Ok(())
        }

        fn delete(&self, key: &S::Key) -> Result<()> {

            let mut store = self.inner.clone().write()?;
            let key =key.encode()?;
            store.remove(&key);
            Ok(())
        }

        fn contains(&self, key: &S::Key) -> Result<bool> {
            let store = self.inner.clone().read()?;
            let key = key.encode()?;
            Ok(store.contains_key(&key))
        }
    }

    struct TempStorage {
        utxo: Arc<UTXO>,
    }

    fn setup_storage(accounts: &Vec<Account>) -> (TempStorage) {
        let coin_base = [0_u8; 32];
        let mut unspent = HashMap::new();

        let res: Result<Vec<_>> = accounts.iter().map(|account| {
            Tx::coinbase(account)
        }).collect();

        let txs = res.unwrap();
        let memstore = Arc::new(MemStore::new());

        let temp = TempStorage {
            utxo: Arc::new(UTXO::new(memstore))
        };


        for tx in txs.iter() {
            temp.utxo.put(tx).unwrap()
        }


        temp
    }



    fn execute_tx(tx: Tx, utxo: &UTXO) -> Result<bool> {
        let valid = validate_transaction(&tx, s)?;
        for t in tx.inputs.iter() {
            utxo.spend(t.prev_tx_index,&t.prev_tx_id);
        }
        utxo.put(&tx);
        Ok(valid)
    }



    #[test]
    fn test_valid_tx() {
        let bob = create_account();
        let alice = create_account();
        let dave = create_account();
        let storage = setup_storage(&vec![bob, alice]);
        // Alice send bob 5 coins

       /* let tx_1 = {
            let tx_in = TxIn::new(coinbase_tx_id, 1, alice.pub_key);
            let tx_out_1 = TxOut::new(alice.pub_key, 5);
            let tx_out_2 = TxOut::new(bob.pub_key, 3);
            let tx_out_3 = TxOut::new(dave.pub_key, 2);
            let tx = Tx::signed(&alice, 10,vec![tx_in], vec![tx_out_1, tx_out_2, tx_out_3]).unwrap();
            let tx_id = tx.tx_id;
            assert_eq!(execute_tx(tx, &mut storage.unspent).unwrap(), true);
            tx_id
        };

        println!("Bob Balance: {}", available_balance(&bob.pub_key, &storage.unspent));
        println!("Alice Balance: {}", available_balance(&alice.pub_key, &storage.unspent));
        println!("Dave Balance: {}", available_balance(&dave.pub_key, &storage.unspent));
        println!("---------------------------------------------------------------------------------------------");

        {
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
