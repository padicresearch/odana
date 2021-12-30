use storage::{KVStore, KVEntry, PersistentStorage};
use crate::transaction::Tx;
use anyhow::Result;
use rusqlite::{params, Connection};
use crate::utxo::{UTXO, UTXOStore};
use itertools::Itertools;
use crate::errors::BlockChainError;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use types::TxHash;
use derive_getters::Getters;
use storage::codec::{Encoder, Decoder};

pub type MemPoolStorageKV = dyn KVStore<MemPool> + Send + Sync;

pub struct MemPoolDB {
    conn: Connection,
}

const INIT_DB: &str =
    r#"
CREATE TABLE IF NOT EXISTS mempool (
    id              VARCHAR(64) NOT NULL PRIMARY KEY,
    fees            INTEGER NOT NULL,
    amount          INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL,
    is_coinbase     BOOLEAN
);
"#;

const COMMAND_INSERT: &str = "INSERT INTO mempool (id, fees, amount, timestamp) VALUES (?1,?2,?3,?4)";
const COMMAND_DELETE: &str = "DELETE FROM mempool WHERE id = ?1;";
const QUERY_ORDER_BY_FEES_DESC: &str = "SELECT id, fees, amount, timestamp, is_coinbase FROM mempool WHERE is_coinbase == false ORDER BY fees DESC, timestamp DESC LIMIT 2500";

impl MemPoolDB {
    pub fn open_in_memory() -> Result<MemPoolDB> {
        let conn = Connection::open_in_memory()?;
        conn.execute(INIT_DB, [])?;
        Ok(MemPoolDB {
            conn
        })
    }

    pub fn open<P: AsRef<Path>>(p: P) -> Result<MemPoolDB> {
        let conn = Connection::open(p)?;
        conn.execute(INIT_DB, [])?;
        Ok(MemPoolDB {
            conn
        })
    }

    pub fn insert(&self, index: MemPoolIndex) -> Result<()> {
        self.conn.execute(COMMAND_INSERT, params![index.id, index.fees, index.amount, index.timestamp])?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn.execute(COMMAND_DELETE, params![id])?;
        Ok(())
    }

    pub fn all_order_by_fees_desc(&self) -> Result<Vec<MemPoolIndex>> {
        let mut stmt = self.conn.prepare(QUERY_ORDER_BY_FEES_DESC)?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            println!("{:?}", id);
            Ok(MemPoolIndex {
                id: row.get(0)?,
                fees: row.get(1)?,
                amount: row.get(2)?,
                timestamp: row.get(3)?,
                is_coinbase: row.get(4)?,
            })
        })?;


        let mut results = Vec::with_capacity(2500);

        for index in rows {
            results.push(index?)
        }

        results.shrink_to_fit();

        Ok(results)
    }
}

pub struct MemPoolIndex {
    id: String,
    fees: i64,
    amount: i64,
    timestamp: i64,
    is_coinbase : bool
}

pub struct MemPool {
    primary: Arc<MemPoolStorageKV>,
    index: MemPoolDB,
    utxo: Arc<UTXO>,
}

#[derive(Serialize, Deserialize, Getters, Debug, Clone)]
pub struct MempoolSnapsot {
    pending: Vec<TxHash>,
    valid: Vec<TxHash>,
}

impl Encoder for MempoolSnapsot {}

impl Decoder for MempoolSnapsot {}


impl MemPool {
    pub fn new(utxo: Arc<UTXO>, storage: Arc<PersistentStorage>, index_path: Option<PathBuf>) -> Result<MemPool> {
        Ok(MemPool {
            primary: {
                match storage.as_ref() {
                    PersistentStorage::MemStore(storage) => {
                        storage.clone()
                    }
                    PersistentStorage::SledDB(storage) => {
                        storage.clone()
                    }
                }
            },
            index: match index_path {
                Some(index_path) => MemPoolDB::open(index_path.as_path()),
                None => MemPoolDB::open_in_memory()
            }?,
            utxo,
        })
    }

    pub fn put(&self, tx: &Tx) -> Result<()> {
        let (in_amount, out_amount) = crate::transaction::calculate_tx_in_out_amount(tx, self.utxo.as_ref())?;
        if self.primary.contains(&tx.tx_id)? {
            return Ok(());
        }
        self.primary.put(tx.tx_id, tx.clone());
        self.index.insert(MemPoolIndex {
            id: hex::encode(tx.tx_id),
            fees: (in_amount as i64).saturating_sub((out_amount as i64)),
            amount: in_amount as i64,
            timestamp: chrono::Utc::now().timestamp(),
            is_coinbase: tx.is_coinbase()
        })
    }

    pub fn contains(&self, tx_id: &[u8; 32]) -> Result<bool> {
        self.primary.contains(tx_id)
    }

    pub fn remove(&self, tx_id: &[u8; 32]) -> Result<()> {
        self.primary.delete(tx_id);
        self.index.delete(&hex::encode(tx_id))?;
        Ok(())
    }

    pub fn get_tx(&self, tx_id: &[u8; 32]) -> Result<Option<Tx>> {
        self.primary.get(tx_id)
    }

    pub fn fetch(&self) -> Result<Vec<Tx>> {
        let iter = self.index.all_order_by_fees_desc()?;
        let results: Result<Vec<_>, _> = iter.iter().map(|index| {
            let tx_id_raw = hex::decode(index.id.clone())?;
            let mut tx_id = [0_u8; 32];
            tx_id.copy_from_slice(&tx_id_raw);
            let value = self.primary.get(&tx_id)?;
            value.ok_or(BlockChainError::MemPoolTransactionNotFound)
        }).collect();
        results.map_err(|e| e.into())
    }

    pub fn snapshot(&self) -> Result<MempoolSnapsot> {
        let valid_transactions: Result<Vec<TxHash>> = self.utxo.iter()?.map(|(k, _)| {
            let key = k?;
            Ok(key.tx_hash)
        }).collect();

        let pending_transactions: Result<Vec<TxHash>> = self.primary.iter()?.map(|(_, v)| {
            let tx = v?;
            Ok(tx.tx_id)
        }).collect();

        Ok(MempoolSnapsot {
            pending: pending_transactions?,
            valid: valid_transactions?,
        })
    }
}

impl KVEntry for  MemPool {
    type Key = [u8; 32];
    type Value = Tx;

    fn column() -> &'static str {
        "mempool"
    }
}

#[cfg(test)]
mod tests {
    use crate::mempool::MemPool;
    use storage::memstore::MemStore;
    use std::sync::Arc;
    use crate::account::create_account;
    use crate::transaction::Tx;
    use anyhow::Result;
    use crate::utxo::UTXO;
    use storage::PersistentStorage::SledDB;
    use storage::{PersistentStorage, KVEntry};
    use crate::block_storage::BlockStorage;
    use crate::blockchain::BlockChainState;

    #[test]
    fn test_mempool() {
        let storage = Arc::new(PersistentStorage::MemStore(Arc::new(MemStore::new(vec![BlockStorage::column(), UTXO::column(), MemPool::column(), BlockChainState::column()]))));
        let utxo = Arc::new(UTXO::new(storage.clone()));
        //let mempool = Arc::new(MemPool::new(utxo,memstore, Some("/home/mambisi/CLionProjects/tuchain/test/mempoolidx.db")).unwrap());
        let mempool = Arc::new(MemPool::new(utxo, storage.clone(), None).unwrap());

        let bob = create_account();
        let alice = create_account();
        let dave = create_account();

        let accounts = vec![bob, alice, dave];
        for account in accounts.iter() {
            mempool.put(&Tx::coinbase(account, 0).unwrap()).unwrap()
        }

        let txs = mempool.fetch().unwrap();
        println!("{:?}", txs.len());

        mempool.remove(&txs[0].tx_id).unwrap();

        let txs = mempool.fetch().unwrap();
        println!("{:?}", txs.len());
    }
}