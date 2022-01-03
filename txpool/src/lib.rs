use types::TxHash;
use transaction::{verify_signed_transaction};
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;
use types::tx::Transaction;

type TxHashRef = Arc<TxHash>;
type TransactionRef = Arc<Transaction>;

pub struct TxPool {
    pending: DashMap<TxHashRef, TransactionRef>,
    queue: DashMap<TxHashRef, TransactionRef>,
}

impl TxPool {
    pub fn add(&self, tx: Transaction) -> Result<()> {
        verify_signed_transaction(&tx)?;
        let tx_hash = tx.hash();
        if self.pending.contains_key(&tx_hash) || self.queue.contains_key(&tx_hash) {
            return Ok(())
        }
        self.queue.insert(Arc::new(tx.hash()), Arc::new(tx));
        Ok(())
    }

    pub fn promote(&self, txs: Vec<TxHashRef>) {
        for tx_hash in txs.iter() {
            if let Some((tx_hash, tx)) = self.queue.remove(tx_hash) {
                self.pending.insert(tx_hash, tx);
            }
        }
    }

    pub fn remove(&self, tx_hash: &TxHash) {
        self.pending.remove(tx_hash);
        self.queue.remove(tx_hash);
    }
}
