mod error;
mod tx_lookup;
mod tx_noncer;
mod txlist;

#[cfg(test)]
mod tests;

use tracing::{debug, info};
use crate::error::TxPoolError;
use crate::tx_lookup::TxLookup;
use crate::tx_noncer::TxNoncer;
use anyhow::{Error, Result};
use dashmap::{DashMap, ReadOnlyView};
use primitive_types::H160;
use std::borrow::BorrowMut;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use traits::{ChainState, StateDB};
use transaction::validate_transaction;
use types::tx::Transaction;
use types::TxHash;
use std::collections::{BTreeMap, BTreeSet};
use types::block::BlockHeader;

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
    head: BlockHeader,

}

pub type TxPoolIterator<'a> = Box<dyn 'a + Send + Iterator<Item=(TxHashRef, TransactionRef)>>;

impl<Chain, State> TxPool<Chain, State>
    where
        Chain: ChainState,
        State: StateDB,
{
    pub fn new(config: TxPoolConfig, chain: Chain, state: State) -> Result<Self> {
        Ok(Self {
            chain,
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup: TxLookup::new()?,
            config,
            head: chain.current_head()?
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
            head: chain.current_head()?
        })
    }

    fn add(&self, tx: Transaction, is_local: bool) -> Result<bool> {
        let tx_hash = Arc::new(tx.hash());
        let tx = Arc::new(tx);

        if self.lookup.contains(&tx_hash) {
            return Err(TxPoolError::TransactionAlreadyKnown.into());
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
            self.lookup
                .add(tx_hash.clone(), tx, is_local, overlaping_tx_is_pending)?;
            return Ok(true);
        }
        // Add transaction to queue
        self.lookup.add(tx_hash, tx, is_local, false)?;
        Ok(false)
    }

    pub fn stats(&self) -> (usize, usize) {
        self.lookup.stats()
    }

    pub fn content(
        &self,
    ) -> Result<(
        BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>,
        BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>,
    )> {
        self.lookup.content()
    }

    pub fn content_from(
        &self,
        address: H160,
    ) -> Result<(
        BTreeMap<TxHashRef, TransactionRef>,
        BTreeMap<TxHashRef, TransactionRef>,
    )> {
        self.lookup.content_from(address)
    }

    pub fn pending(&self) -> Result<BTreeMap<H160, BTreeMap<TxHashRef, TransactionRef>>> {
        self.lookup.pending()
    }

    pub fn locals(&self) -> Result<BTreeSet<H160>> {
        self.lookup.locals()
    }

    pub fn reset(&self, new_head: &BlockHeader) -> Result<()> {
        if self.head.block_hash() != new_head.block_hash() {
            let depth = ((self.head.level() as f64) - (new_head.level() as f64)).abs() as u64;
            if depth > 64 {
                info!(depth = depth, "Skipped deep transaction packing")
            } else {
                let rem = self.chain.get_block()
            }
        }

        todo!()
    }


    /// Takes transaction form queue and adds them to pending
    fn package(&self) {
        // Remove transactions with nonce lower than current account state
        // Remove transactions that are too costly ( sender cannot fulfil transaction)
        // Get all remaining transactions and promote them
    }

    fn promote_executables(&self) {}

    pub fn nonce(&self, address: &H160) -> u64 {
        self.pending_nonces.get(address)
    }

    /// Remove transaction form pending and queue
    /// This occurs when a new block
    pub fn remove(&self, tx_hash: &TxHash) {
        self.reorg()
    }
}
