mod error;
mod tx_lookup;
mod tx_noncer;
mod txlist;

#[cfg(test)]
mod tests;

use crate::tx_lookup::TxLookup;
use crate::tx_noncer::TxNoncer;
use anyhow::{Error, Result};
use dashmap::{DashMap, ReadOnlyView};
use std::borrow::BorrowMut;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use traits::{BlockchainState, StateDB};
use transaction::validate_transaction;
use types::tx::Transaction;
use types::TxHash;
use crate::error::TxPoolError;
use primitive_types::H160;

type TxHashRef = Arc<TxHash>;
type TransactionRef = Arc<Transaction>;

// TODO: truncate Pending transactions
#[derive(Clone)]
pub struct TxPoolConfig {
    transaction_limit: usize,
}

impl Default for TxPoolConfig {
    fn default() -> Self {
        TxPoolConfig {
            transaction_limit: 2048,
        }
    }
}

pub struct TxPool<Chain, State> {
    chain: Chain,
    state: State,
    pending_nonces: TxNoncer<State>,
    lookup: TxLookup,
    config: TxPoolConfig,
}

pub type TxPoolIterator<'a> = Box<dyn 'a + Send + Iterator<Item=(TxHashRef, TransactionRef)>>;

impl<Chain, State> TxPool<Chain, State>
    where
        Chain: BlockchainState,
        State: StateDB,
{
    pub fn new(config: TxPoolConfig, chain: Chain, state: State) -> Result<Self> {
        Ok(Self {
            chain,
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup: TxLookup::new()?,
            config,
        })
    }

    #[cfg(test)]
    pub fn new_lookup(
        lookup: TxLookup,
        config: TxPoolConfig,
        chain: Chain,
        state: State,
    ) -> Result<Self> {
        Ok(Self {
            chain,
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup,
            config,
        })
    }

    fn add(&self, tx: Transaction, is_local: bool) -> Result<bool> {
        let tx_hash = Arc::new(tx.hash());
        let tx = Arc::new(tx);

        if self.lookup.contains(&tx_hash) {
            return Err(TxPoolError::TransactionAlreadyKnown.into())
        }

        match validate_transaction(&tx, None, None) {
            Ok(_) => {}
            Err(error) => {
                return Err(error);
            }
        }

        if self.lookup.count() + 1 > self.config.transaction_limit {
            let old_tx = self.lookup.get_lowest_priced(tx.fees())?;
            match old_tx {
                None => {
                    println!("Discarding Tx {:?}", tx_hash)
                }
                Some(old_tx) => {
                    self.lookup.delete(&old_tx.hash())?;
                    self.lookup.add(tx_hash, tx, is_local, false)?;
                    return Ok(true);
                }
            }
        }

        let overlaping_tx = self
            .lookup
            .get_overlap_pending_tx(tx.sender_address(), tx.nonce())?;
        if let Some((overlaping_tx, overlaping_tx_is_pending, _)) = overlaping_tx {
            let overlaping_tx_hash = overlaping_tx.hash();
            self.lookup.delete(&overlaping_tx_hash)?;
            self.lookup.add(tx_hash.clone(), tx, is_local, overlaping_tx_is_pending)?;
            return Ok(true);
        }
        // Add transaction to queue
        self.lookup.add(tx_hash, tx, is_local, false)?;
        Ok(false)
    }
    /// Takes transaction form queue and adds them to pending
    fn reorg(&self) {}

    pub fn nonce(&self, address: &H160) -> u64 {
        self.pending_nonces.get(address)
    }

    /// Remove transaction form pending and queue
    /// This occurs when a new block
    pub fn remove(&self, tx_hash: &TxHash) {
        self.reorg()
    }

    // pub fn queue(&self) -> TxPoolIterator {
    //     Box::new(self.queue.iter().map(|kv| {
    //         (kv.key().clone(), kv.value().clone())
    //     }))
    // }
    //
    // pub fn pending(&self) -> TxPoolIterator {
    //     Box::new(self.pending.iter().map(|kv| {
    //         (kv.key().clone(), kv.value().clone())
    //     }))
    // }
}
