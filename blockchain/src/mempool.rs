use storage::{Storage, KVEntry};
use crate::transaction::Tx;
use anyhow::Result;
use rusqlite::{params, Connection};
use crate::utxo::UTXO;
use itertools::Itertools;
use crate::errors::BlockChainError;
use std::sync::Arc;
use std::path::{Path, PathBuf};

pub type MemPoolStorageKV = dyn Storage<MemPool> + Send + Sync;

pub struct MemPoolDB {
    conn: Connection,
}

const INIT_DB: &str =
    r#"
CREATE TABLE IF NOT EXISTS mempool (
    id              VARCHAR(64) NOT NULL PRIMARY KEY,
    fees            INTEGER NOT NULL,
    amount          INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL
);
"#;

const COMMAND_INSERT: &str = "INSERT INTO mempool (id, fees, amount, timestamp) VALUES (?1,?2,?3,?4)";
const COMMAND_DELETE: &str = "DELETE FROM mempool WHERE id = ?1;";
const QUERY_ORDER_BY_FEES_DESC: &str = "SELECT id, fees, amount, timestamp FROM mempool ORDER BY fees DESC, timestamp DESC LIMIT 2500";

impl MemPoolDB {
    pub fn open_in_memory() -> Result<MemPoolDB> {
        let conn = Connection::open_in_memory()?;
        conn.execute(INIT_DB, [])?;
        Ok(MemPoolDB {
            conn
        })
    }

    pub fn open<P :AsRef<Path>>(p : P) -> Result<MemPoolDB> {
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
            let id : String = row.get(0)?;
            println!("{:?}",  id);
            Ok(MemPoolIndex {
                id: row.get(0)?,
                fees: row.get(1)?,
                amount: row.get(2)?,
                timestamp: row.get(3)?,
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
}

pub struct MemPool {
    primary: Arc<MemPoolStorageKV>,
    index: MemPoolDB,
    utxo : Arc<UTXO>
}


impl MemPool {
    pub fn new(utxo : Arc<UTXO>, primary: Arc<MemPoolStorageKV>, index_path : Option<PathBuf> ) -> Result<MemPool> {
        Ok(MemPool {
            primary,
            index: match index_path {
                Some(index_path) => MemPoolDB::open(index_path.as_path()),
                None => MemPoolDB::open_in_memory()
            }?,
            utxo
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
        })
    }

    pub fn remove(&self, tx_id: &[u8; 32]) -> Result<()> {
        self.primary.delete(tx_id);
        self.index.delete(&hex::encode(tx_id))?;
        Ok(())
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
}

impl KVEntry for MemPool {
    type Key = [u8; 32];
    type Value = Tx;
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

    #[test]
    fn test_mempool() {
        let memstore = Arc::new(MemStore::new());
        let memstore2 = Arc::new(MemStore::new());
        let utxo = Arc::new(UTXO::new(memstore2));
        //let mempool = Arc::new(MemPool::new(utxo,memstore, Some("/home/mambisi/CLionProjects/tuchain/test/mempoolidx.db")).unwrap());
        let mempool = Arc::new(MemPool::new(utxo,memstore, None).unwrap());

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