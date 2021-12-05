use storage::{KVEntry, Storage};
use storage::codec::{Encoder, Decoder};
use std::io::{Cursor, Read};
use crate::transaction::{Tx, TxOut};
use std::sync::Arc;
use anyhow::Result;
use crate::errors::BlockChainError;
use serde::{Serialize, Deserialize};

pub type UTXOStorageKV = dyn Storage<UTXO> + Send + Sync;

const ERROR_MSG_KEY_EXISTS: &str = "Key already exist in utxo";
const ERROR_MSG_COIN_NOT_FOUND: &str = "Spendable output not found";


pub struct UTXO {
    kv : Arc<UTXOStorageKV>
}

impl UTXO {
    pub fn new(storage : Arc<UTXOStorageKV>) -> Self {
        Self {
            kv: storage
        }
    }
}

pub trait UTXOStore {
    fn put(&self, tx : &Tx) -> Result<()>;
    fn spend(&self, index : u16, tx_hash : &[u8;32]) -> Result<()>;
    fn get_coin(&self, index : u16, tx_hash : &[u8;32]) -> Result<Option<CoinOut>>;
    fn contains(&self, index: u16, tx_hash: &[u8; 32]) -> Result<bool>;
}

impl UTXOStore for UTXO {
    fn put(&self, tx: &Tx) -> Result<()> {
        for (index, tx_out) in tx.outputs.iter().enumerate() {
            let key = CoinKey::new(index as u16, tx.tx_id);
            if self.kv.contains(&key)? {
                return Err(BlockChainError::UTXOError(ERROR_MSG_KEY_EXISTS).into())
            }

            self.kv.put(key, CoinOut::new(tx_out.clone()));
        }
        Ok(())
    }

    fn spend(&self, index: u16, tx_hash: &[u8; 32]) -> Result<()> {
        let key = CoinKey::new(index as u16, *tx_hash);
        let mut coin = self.kv.get(&key)?.ok_or(BlockChainError::UTXOError(ERROR_MSG_COIN_NOT_FOUND))?;
        coin.spend();
        self.kv.put(key,coin)
    }

    fn get_coin(&self, index: u16, tx_hash: &[u8; 32]) -> Result<Option<CoinOut>> {
        let key = CoinKey::new(index as u16, *tx_hash);
        self.kv.get(&key)
    }

    fn contains(&self, index: u16, tx_hash: &[u8; 32]) -> Result<bool> {
        let key = CoinKey::new(index as u16, *tx_hash);
        self.kv.contains(&key)
    }
}



#[derive(Clone)]
pub struct CoinKey {
    pub tx_hash : [u8;32],
    pub index : u16
}

impl CoinKey {
    fn new(index : u16, tx_hash : [u8;32]) -> Self {
        CoinKey {
            tx_hash,
            index
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CoinOut {
    pub tx_out : TxOut,
    pub is_spent : bool
}

impl CoinOut {
    pub(crate) fn new(tx_out : TxOut) -> Self{
        CoinOut {
            tx_out,
            is_spent: false
        }
    }

    pub fn spend(&mut self) {
        self.is_spent = true
    }
}

impl Encoder for CoinKey {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(34);
        buf.extend_from_slice(&self.tx_hash);
        buf.extend_from_slice(&self.index.to_be_bytes());
        Ok(buf)
    }
}


impl Decoder for CoinKey {
    fn decode(buf: &[u8]) -> Result<Self> {

        let mut cursor = Cursor::new(buf);
        let mut tx_hash = [0_u8; 32];
        let mut raw_index = [0_u8; 2];

        cursor.read_exact(&mut tx_hash);
        cursor.read_exact(&mut raw_index);

        let index = u16::from_be_bytes(raw_index);

        Ok(CoinKey {
            tx_hash,
            index
        })

    }
}

impl Encoder for CoinOut {
    fn encode(&self) ->Result<Vec<u8>> {
        bincode::serialize(&self).map_err(|e|BlockChainError::DeserializationError(e).into())
    }
}


impl Decoder for CoinOut {
    fn decode(buf: &[u8]) -> Result<Self> {
        bincode::deserialize(buf).map_err(|e|BlockChainError::DeserializationError(e).into())
    }
}




impl KVEntry for UTXO {
    type Key = CoinKey;
    type Value = CoinOut;
}