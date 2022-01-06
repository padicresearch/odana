use crate::error::TxPoolError;
use crate::{TransactionRef, TxHashRef};
use anyhow::Result;
use dashmap::DashMap;
use itertools::Itertools;
use primitive_types::H160;
use rusqlite::{Connection, Error, MappedRows, Row, ToSql, Statement};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::{Arc, Mutex};
use types::{AccountId, TxHash};

// const CREATE_TXPOOL_TABLE: &str = r#"
// CREATE TABLE IF NOT EXISTS txpool (
//     id              VARCHAR(64) NOT NULL PRIMARY KEY,
//     fees            BLOB NOT NULL,
//     nonce           INTEGER NOT NULL,
//     address         VARCHAR(64) NOT NULL,
//     is_local        BOOLEAN NOT NULL,
//     is_pending      BOOLEAN NOT NULL DEFAULT false
// );
// "#;

const RESET_TXPOOL_TABLE: &str = r#"
BEGIN;
DROP INDEX IF EXISTS index_fees;
DROP INDEX IF EXISTS index_address;
DROP TABLE IF EXISTS txpool;
CREATE TABLE txpool (
    id              VARCHAR(64) NOT NULL PRIMARY KEY,
    fees            BLOB NOT NULL,
    nonce           INTEGER NOT NULL,
    address         VARCHAR(64) NOT NULL,
    is_local        BOOLEAN NOT NULL,
    is_pending      BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX index_fees ON txpool(fees);
CREATE INDEX index_address ON txpool(address);
COMMIT;
"#;

/// Inserts an new transaction into the index
const PROMOTE_TX_TEMPLATE: &str =
    "UPDATE txpool SET is_pending = true WHERE id == {} AND is_pending == false;";

/// Inserts an new transaction into the index
const PROMOTE_TX_STMT: &str =
    "UPDATE txpool SET is_pending = true WHERE id == ?1 AND is_pending == false;";

/// Get an overlap tx [`:threshold` , `:address`]
const READY_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE nonce < :threshold AND address == :address AND is_pending == false;";

/// Get transaction that cannot be paid tx [`:cost_limit` , `:address`]
const UNPAYABLE_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE fees > :cost_limit AND address == :address AND is_pending == false;";
/// Inserts an new transaction into the index
const INSERT_TX: &str =
    "INSERT INTO txpool (id, fees, nonce, address, is_local) VALUES (:id,:fees,:nonce,:address,:is_local)";

/// Get all transaction belonging to an address which does not meet the threshold nonce params [`:address` , `:current_nonce`]
const FORWARD_TX: &str =
    "SELECT id, fees, nonce, address, is_local FROM txpool WHERE address == :address AND nonce < :current_nonce AND is_pending == false;";
/// Remove all transaction belonging to an address which does not meet the threshold nonce
const DELETE_FORWARD_TX: &str =
    "DELETE FROM txpool WHERE address == :address AND nonce < :current_nonce;";
/// Get the transaction with the lowest fees
const QUERY_LOWEST_PRICED_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE feed < :threshold AND is_pending == false ORDER BY fees LIMIT 1;";
const QUERY_LOWEST_PRICED_REMOTE_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE feed < :threshold AND is_local == false LIMIT 1;";
const QUERY_LOWEST_PRICED_LOCAL_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE feed < :threshold AND is_local == true LIMIT 1;";

/// Get an overlap tx [`:nonce` , `:address`]
const GET_OVERLAP_TX: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE nonce == :nonce AND address == :address LIMIT 1;";

const GET_PENDING: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_pending == true ORDER BY nonce GROUP BY address;";

const GET_QUEUE: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_pending == false ORDER BY nonce GROUP BY address;";

const GET_CONTENT: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool ORDER BY nonce GROUP BY address;";

const GET_CONTENT_BY: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool ORDER BY nonce WHERE address == :address;";

const COUNT_GET_PENDING: &str = "SELECT COUNT(id) FROM txpool WHERE is_pending == true;";
const COUNT_GET_QUEUE: &str = "SELECT COUNT(id) FROM txpool WHERE is_pending == false;";

const GET_LOCALS: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == true ORDER BY nonce GROUP BY address;";
const GET_LOCALS_PENDING: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == true AND is_pending == true ORDER BY nonce GROUP BY address;";
const GET_LOCALS_QUEUE: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == false AND is_pending == false ORDER BY nonce GROUP BY address;";

const GET_REMOTES: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == false ORDER BY nonce GROUP BY address;";
const GET_REMOTES_PENDING: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == false AND is_pending == true ORDER BY nonce GROUP BY address;";
const GET_REMOTES_QUEUE: &str =
    "SELECT id, fees, nonce, address, is_local,is_pending FROM txpool WHERE is_local == false AND is_pending == false ORDER BY nonce GROUP BY address;";

/// Delete transaction with `:id`
const DELETE_TX: &str = "DELETE FROM txpool WHERE id = ?1;";

const DELETE_MULTIPLE_TX: &str = "DELETE FROM txpool WHERE id in ?1;";

pub struct TxIndexRow {
    id: String,
    fees: i128,
    nonce: i64,
    address: String,
    is_local: bool,
    is_pending: bool,
}

impl TxIndexRow {
    pub fn new(tx: &TransactionRef, is_local: bool) -> Self {
        Self {
            id: hex::encode(tx.hash()),
            fees: tx.fees() as i128,
            nonce: tx.nonce_u32() as i64,
            address: tx.sender_address().to_string(),
            is_local,
            is_pending: false,
        }
    }
    fn as_sql_param(&self) -> Vec<(&str, &dyn ToSql)> {
        vec![
            (":id", &self.id),
            (":nonce", &self.nonce),
            (":fees", &self.fees),
            (":address", &self.address),
            (":is_local", &self.is_local),
        ]
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            fees: row.get(1)?,
            nonce: row.get(2)?,
            address: row.get(3)?,
            is_local: row.get(4)?,
            is_pending: row.get(5)?,
        })
    }
}

pub struct TxLookup {
    mu: Mutex<()>,
    conn: Connection,
    txs: Arc<DashMap<TxHashRef, TransactionRef>>,
    senders: Arc<DashMap<TxHashRef, H160>>,
}

impl TxLookup {
    fn conv_txhash<'a>(
        rows: Box<dyn 'a + Iterator<Item=rusqlite::Result<TxIndexRow, rusqlite::Error>>>,
    ) -> Result<Vec<[u8; 32]>> {
        let mut tx_hashes = Vec::new();

        for row in rows {
            let index = row?;
            let tx_id_raw = hex::decode(index.id)?;
            let mut tx_id = [0_u8; 32];
            tx_id.copy_from_slice(&tx_id_raw);
            tx_hashes.push(tx_id)
        }
        Ok(tx_hashes)
    }

    fn conv_tx<'a>(
        &'a self,
        rows: Box<dyn 'a + Iterator<Item=rusqlite::Result<TxIndexRow, rusqlite::Error>>>,
    ) -> Box<dyn 'a + Iterator<Item=Result<(TxHashRef, TransactionRef, bool, bool), TxPoolError>>>
    {
        let mut iter_tx_hash = rows.map(|index| {
            index
                .map_err(|e| TxPoolError::SqliteError(e))
                .and_then(|index| {
                    hex::decode(&index.id)
                        .map(|out| (out, index.is_pending, index.is_local))
                        .map_err(|hex_error| TxPoolError::HexError(hex_error))
                })
                .and_then(|(tx_id_raw, is_pending, is_local)| {
                    let mut tx_id = [0_u8; 32];
                    tx_id.copy_from_slice(&tx_id_raw);
                    Ok((tx_id, is_pending, is_local))
                })
        });

        let txs = self.txs.clone();

        let result = iter_tx_hash.map(move |tx_hash| {
            tx_hash
                .and_then(|(tx_hash, is_pending, is_local)| {
                    txs.get(&tx_hash)
                        .ok_or(TxPoolError::TransactionNotFoundInPrimary)
                        .and_then(|r| {
                            Ok((r.key().clone(), r.value().clone(), is_pending, is_local))
                        })
                })
                .map_err(|e| TxPoolError::GenericError(e.into()))
        });

        Box::new(result)
    }
}

impl TxLookup {
    pub(crate) fn new() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(RESET_TXPOOL_TABLE)?;
        Ok(Self {
            mu: Mutex::new(()),
            conn,
            txs: Arc::new(Default::default()),
            senders: Arc::new(Default::default()),
        })
    }

    #[cfg(test)]
    pub(crate) fn new_in_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(RESET_TXPOOL_TABLE)?;
        Ok(Self {
            mu: Mutex::new(()),
            conn,
            txs: Arc::new(Default::default()),
            senders: Arc::new(Default::default()),
        })
    }

    pub(crate) fn add(&self, tx_hash: TxHashRef, tx: TransactionRef, is_local: bool) -> Result<()> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let index_row = TxIndexRow::new(&tx, is_local);
        self.conn
            .execute(INSERT_TX, index_row.as_sql_param().as_slice())?;
        self.txs.insert(tx_hash.clone(), tx.clone());
        self.senders.insert(tx_hash, tx.sender_address());
        println!("new transaction {} added to index", index_row.id);
        Ok(())
    }

    pub(crate) fn contains(&self, tx_hash: &TxHash) -> bool {
        self.txs.contains_key(tx_hash)
    }

    pub(crate) fn promote(&self, txs: Vec<TxHash>) -> Result<()> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut stmt = String::from("BEGIN;");
        for tx in txs {
            stmt.push_str(&format!(
                "UPDATE txpool SET is_pending = true WHERE id == {};",
                hex::encode(tx)
            ));
        }
        stmt.push_str("COMMIT;");
        self.conn.execute_batch(stmt.as_str()).map_err(|e| e.into())
    }
    pub(crate) fn delete(&self, tx_hash: &TxHash) -> Result<()> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        self.delete_index(&hex::encode(tx_hash))?;
        self.txs.remove(tx_hash);
        self.senders.remove(tx_hash);
        Ok(())
    }

    pub(crate) fn forward(&self, address: H160, current_nonce: u64) -> Result<Vec<TxHash>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut stmt = self.conn.prepare(FORWARD_TX)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":current_nonce" : current_nonce as i64,
                ":account" : address.to_string()
            },
            |row| TxIndexRow::from_row(row),
        )?;
        TxLookup::conv_txhash(Box::new(rows))
    }

    pub(crate) fn unpayable(&self, address: H160, cost_limit: u128) -> Result<Vec<TxHash>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut stmt = self.conn.prepare(UNPAYABLE_TX)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":cost_limit" : cost_limit.to_be_bytes().to_vec(),
                ":account" : address.to_string()
            },
            |row| TxIndexRow::from_row(row),
        )?;
        TxLookup::conv_txhash(Box::new(rows))
    }

    pub(crate) fn ready(&self, address: H160, current_nonce: u64) -> Result<Vec<TxHash>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut stmt = self.conn.prepare(READY_TX)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":current_nonce" : current_nonce as i64,
                ":account" : address.to_string()
            },
            |row| TxIndexRow::from_row(row),
        )?;
        TxLookup::conv_txhash(Box::new(rows))
    }

    pub(crate) fn reset(&self) -> Result<()> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        self.conn
            .execute_batch(RESET_TXPOOL_TABLE)
            .map_err(|e| e.into())
    }

    pub(crate) fn delete_index(&self, id: &str) -> Result<()> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        self.conn.execute(DELETE_TX, rusqlite::params![id])?;
        Ok(())
    }

    pub(crate) fn count(&self) -> usize {
        self.txs.len()
    }

    pub(crate) fn get_overlap_pending_tx(
        &self,
        address: H160,
        nonce: u64,
    ) -> Result<Option<(TransactionRef, bool, bool)>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut stmt = self.conn.prepare(GET_OVERLAP_TX)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":nonce" : nonce as i64,
                ":address" : address.to_string()
            },
            |row| TxIndexRow::from_row(row),
        )?;
        match rows.next() {
            None => Ok(None),
            Some(row) => {
                let index = row?;
                let tx_id_raw = hex::decode(index.id.clone()).map_err(|e| TxPoolError::from(e))?;
                let mut tx_id = [0_u8; 32];
                tx_id.copy_from_slice(&tx_id_raw);
                Ok(self
                    .txs
                    .get(&tx_id)
                    .map(|kv| (kv.value().clone(), index.is_pending, index.is_local)))
            }
        }
    }

    fn pending_count(&self) -> usize {
        let mut stmt = match self.conn.prepare(COUNT_GET_PENDING) {
            Ok(stmt) => { stmt }
            Err(_) => {
                return 0
            }
        };
        let mut res = stmt.query_map([],
                                     |row| {
                                         let count: i64 = row.get(0).unwrap_or(0);
                                         Ok(count)
                                     },
        );
        match res {
            Ok(mut res) => {
                res.next().unwrap_or(Ok(0)).unwrap_or(0) as usize
            }
            Err(_) => {
                return 0
            }
        }
    }
    fn queue_count(&self) -> usize {
        let mut stmt = match self.conn.prepare(COUNT_GET_QUEUE) {
            Ok(stmt) => { stmt }
            Err(_) => {
                return 0
            }
        };
        let mut res = stmt.query_map([],
                                     |row| {
                                         let count: i64 = row.get(0).unwrap_or(0);
                                         Ok(count)
                                     },
        );
        match res {
            Ok(mut res) => {
                res.next().unwrap_or(Ok(0)).unwrap_or(0) as usize
            }
            Err(_) => {
                return 0
            }
        }
    }

    pub(crate) fn stats(&self) -> (usize, usize) {
        (self.pending_count(), self.queue_count())
    }

    pub(crate) fn content(
        &self,
    ) -> Result<(
        BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>,
        BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>,
    )> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut pending: BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>> = Default::default();
        let mut queue: BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>> = Default::default();
        let mut stmt = self.conn.prepare(GET_CONTENT)?;
        let mut rows = stmt.query_map([], |row| TxIndexRow::from_row(row))?;

        let txs = self.conv_tx(Box::new(rows));
        for tx in txs {
            let (tx_hash, tx, is_pending, _) = tx?;
            let mut list = if is_pending {
                pending
                    .entry(tx.sender_address())
                    .or_insert(Default::default())
            } else {
                queue
                    .entry(tx.sender_address())
                    .or_insert(Default::default())
            };
            list.insert(tx_hash, tx);
        }

        Ok((pending, queue))
    }

    pub(crate) fn content_from(
        &self,
        address: H160,
    ) -> Result<(
        BTreeMap<TxHashRef, TransactionRef>,
        BTreeMap<TxHashRef, TransactionRef>,
    )> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut pending: BTreeMap<TxHashRef, TransactionRef> = Default::default();
        let mut queue: BTreeMap<TxHashRef, TransactionRef> = Default::default();
        let mut stmt = self.conn.prepare(GET_CONTENT_BY)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":address" : address.to_string()
            },
            |row| TxIndexRow::from_row(row),
        )?;

        let txs = self.conv_tx(Box::new(rows));
        for tx in txs {
            let (tx_hash, tx, is_pending, _) = tx?;
            if is_pending {
                pending.insert(tx_hash, tx)
            } else {
                queue.insert(tx_hash, tx)
            };
        }
        Ok((pending, queue))
    }

    pub(crate) fn pending(&self) -> Result<BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut pending: BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>> = Default::default();
        let mut stmt = self.conn.prepare(GET_PENDING)?;
        let mut rows = stmt.query_map([], |row| TxIndexRow::from_row(row))?;

        let txs = self.conv_tx(Box::new(rows));
        for tx in txs {
            let (tx_hash, tx, is_pending, _) = tx?;
            if is_pending {
                let list = pending
                    .entry(tx.sender_address())
                    .or_insert(Default::default());
                list.insert(tx_hash, tx);
            }
        }

        Ok(pending)
    }

    pub(crate) fn locals(&self) -> Result<BTreeSet<H160>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let mut locals: BTreeSet<H160> = Default::default();
        let mut stmt = self.conn.prepare(GET_LOCALS)?;
        let mut rows = stmt.query_map([], |row| TxIndexRow::from_row(row))?;

        let txs = self.conv_tx(Box::new(rows));
        for tx in txs {
            let (tx_hash, tx, _, is_local) = tx?;
            let sender = self
                .senders
                .get(&tx_hash)
                .map(|r| r.value().clone())
                .unwrap_or(tx.sender_address());
            if is_local && !locals.contains(&sender) {
                locals.insert(sender);
            }
        }
        Ok((locals))
    }

    pub(crate) fn get_lowest_priced(&self, threshold: u128) -> Result<Option<TransactionRef>> {
        self.mu
            .lock()
            .map_err(|e| TxPoolError::MutexGuardError(format!("{}", e)))?;
        let threshold_blob = threshold.to_be_bytes().to_vec();
        let mut stmt = self.conn.prepare(QUERY_LOWEST_PRICED_TX)?;
        let mut rows = stmt.query_map(
            rusqlite::named_params! {
                ":threshold" : threshold_blob
            },
            |row| TxIndexRow::from_row(row),
        )?;
        match rows.next() {
            None => Ok(None),
            Some(row) => {
                let index = row?;
                let tx_id_raw = hex::decode(index.id.clone()).map_err(|e| TxPoolError::from(e))?;
                let mut tx_id = [0_u8; 32];
                tx_id.copy_from_slice(&tx_id_raw);
                Ok(self.txs.get(&tx_id).map(|kv| kv.value().clone()))
            }
        }
    }
}
