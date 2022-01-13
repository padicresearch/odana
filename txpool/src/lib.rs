mod error;
mod tx_lookup;
mod tx_noncer;
mod txlist;

#[cfg(test)]
mod tests;

use crate::error::TxPoolError;
use crate::tx_lookup::TxLookup;
use crate::tx_noncer::TxNoncer;
use anyhow::{Error, Result};
use dashmap::{DashMap, ReadOnlyView};
use primitive_types::H160;
use std::borrow::BorrowMut;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tracing::tracing_subscriber::fmt::writer::EitherWriter::B;
use tracing::{debug, error, info, warn};
use traits::{ChainState, StateDB};
use transaction::validate_transaction;
use types::block::{Block, BlockHeader};
use types::tx::Transaction;
use types::{Hash, TxHash};

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

type Address = H160;
pub struct TxPool<Chain>
    where
        Chain: ChainState,
{
    chain: Arc<Chain>,
    state: Arc<dyn StateDB>,
    pending_nonces: TxNoncer,
    lookup: TxLookup,
    config: TxPoolConfig,
    head: BlockHeader,
    accounts: BTreeSet<Address>,
}

pub type TxPoolIterator<'a> = Box<dyn 'a + Send + Iterator<Item=(TxHashRef, TransactionRef)>>;

impl<Chain> TxPool<Chain>
    where
        Chain: ChainState,
{
    pub fn new(config: TxPoolConfig, chain: Arc<Chain>, state: Arc<dyn StateDB>) -> Result<Self> {
        Ok(Self {
            chain: chain.clone(),
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup: TxLookup::new()?,
            config,
            head: chain.current_head()?,
            accounts: Default::default(),
        })
    }

    #[cfg(test)]
    pub fn new_lookup(
        lookup: TxLookup,
        config: TxPoolConfig,
        chain: Arc<Chain>,
        state: Arc<dyn StateDB>,
    ) -> Result<Self> {
        Ok(Self {
            chain: chain.clone(),
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup,
            config,
            head: chain.current_head()?,
            accounts: Default::default(),
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

    pub fn reset(&mut self, old_head: Option<BlockHeader>, new_head: BlockHeader) -> Result<()> {
        let mut reinject = HashSet::new();
        // reinject all dropped transactions
        if let Some(old_head) = old_head {
            if old_head.block_hash() != new_head.block_hash() {
                let old_level = *old_head.level();
                let new_level = *new_head.level();

                let depth = ((old_level as f64) - (new_level as f64)).abs() as u64;
                if depth > 64 {
                    info!(depth = depth, "Skipped deep transaction packing")
                } else {
                    let mut discarded = HashSet::new();
                    let mut included = HashSet::new();
                    let mut rem = self.chain.get_block(old_head.block_hash())?;
                    let mut add = self
                        .chain
                        .get_block(new_head.block_hash())?
                        .ok_or(anyhow::anyhow!("new block not found"))?;
                    if let Some(mut rem) = rem {
                        while rem.level() > add.level() {
                            discarded.extend(rem.transactions().into_iter().cloned());
                            rem = match self.chain.get_block(rem.parent_hash())? {
                                None => {
                                    error!(
                                        block = old_head.level(),
                                        hash = hex::encode(old_head.block_hash()).as_str(),
                                        "Unrooted old chain seen by tx pool"
                                    );
                                    return Ok(());
                                }
                                Some(rem) => rem,
                            }
                        }
                        while add.level() > rem.level() {
                            included.extend(add.transactions().into_iter().cloned());
                            add = match self.chain.get_block(add.parent_hash())? {
                                None => {
                                    error!(
                                        block = old_head.level(),
                                        hash = hex::encode(old_head.block_hash()).as_str(),
                                        "Unrooted new chain seen by tx pool"
                                    );
                                    return Ok(());
                                }
                                Some(rem) => rem,
                            }
                        }

                        while add.level() != rem.level() {
                            included.extend(add.transactions().into_iter().cloned());
                            add = match self.chain.get_block(add.parent_hash())? {
                                None => {
                                    error!(
                                        block = old_head.level(),
                                        hash = hex::encode(old_head.block_hash()).as_str(),
                                        "Unrooted new chain seen by tx pool"
                                    );
                                    return Ok(());
                                }
                                Some(block) => block,
                            };
                            discarded.extend(rem.transactions().into_iter().cloned());
                            rem = match self.chain.get_block(rem.parent_hash())? {
                                None => {
                                    error!(
                                        block = old_head.level(),
                                        hash = hex::encode(old_head.block_hash()).as_str(),
                                        "Unrooted old chain seen by tx pool"
                                    );
                                    return Ok(());
                                }
                                Some(block) => block,
                            };
                        }

                        reinject = included
                            .intersection(&discarded)
                            .into_iter()
                            .map(|tx| tx.clone().clone())
                            .collect();
                    } else {
                        if new_level >= old_level {
                            warn!("Transaction pool reset with missing oldhead");
                            return Ok(());
                        }
                        debug!(
                            old = hex::encode(old_head.block_hash()).as_str(),
                            new = hex::encode(new_head.block_hash()).as_str(),
                            "Skipping transaction reset caused by setHead"
                        );
                    }
                }
            }
        }
        let statedb = self.chain.get_state_at(new_head.state_root())?;
        self.state = statedb.clone();
        self.pending_nonces = TxNoncer::new(statedb);
        self.add_txs_locked(Box::new(reinject.into_iter()), false)?;
        Ok(())
    }

    fn add_txs_locked(
        &mut self,
        txs: Box<dyn Iterator<Item=Transaction>>,
        local: bool,
    ) -> Result<BTreeSet<Address>> {
        let mut accounts = BTreeSet::new();
        for tx in txs {
            if !self.add(tx.clone(), local)? {
                accounts.insert(tx.sender_address());
            }
        }
        Ok(accounts)
    }

    fn add_txs(&mut self, txs: Vec<Transaction>, local: bool) -> Result<()> {
        let mut valid_txs = Vec::new();
        for tx in txs {
            if !self.lookup.contains(&tx.hash()) {
                valid_txs.push(tx);
            }
        }
        let accounts = self.add_txs_locked(Box::new(valid_txs.into_iter()), local)?;
        self.promote_executables(accounts)?;
        Ok(())
    }

    /// Takes transaction form queue and adds them to pending
    pub fn package(&self) -> Result<Vec<TransactionRef>> {
        self.lookup.pending_flatten()
    }

    pub fn get(&self, hash: &Hash) -> Option<TransactionRef> {
        self.lookup.all().get(hash).map(|res| res.value().clone())
    }

    pub fn has(&self, hash: &Hash) -> bool {
        self.lookup.all().contains_key(hash)
    }
    fn promote_executables(&self, accounts: BTreeSet<Address>) -> Result<()> {
        //let mut promoted = BTreeSet::new();
        for address in accounts {
            // Remove transactions with nonce lower than current account state
            let forwards = self
                .lookup
                .forward(&address, self.state.account_nonce(&address))?;
            for tx in forwards.iter() {
                self.lookup.delete(tx)?;
            }
            // Remove transactions that are too costly ( sender cannot fulfil transaction)
            let drops = self
                .lookup
                .unpayable(&address, self.state.account_state(&address).free_balance)?;
            for tx in drops.iter() {
                self.lookup.delete(tx)?;
            }
            // Get all remaining transactions and promote them
            let readies = self.lookup.ready(&address, self.nonce(&address))?;
            let readies_count = readies.len();
            self.lookup
                .promote(readies.iter().map(|tx| tx.hash()).collect())?;

            for tx in readies.iter() {
                self.pending_nonces.set(tx.sender_address(), tx.nonce() + 1);
            }
            info!(count = readies_count, "Promoted queued transactions");
        }

        Ok(())
    }

    pub fn nonce(&self, address: &H160) -> u64 {
        self.pending_nonces.get(address)
    }

    pub fn add_remote(&mut self, tx: Transaction) -> Result<()> {
        self.add_txs(vec![tx], false)
    }

    pub fn add_remotes(&mut self, txs: Vec<Transaction>) -> Result<()> {
        self.add_txs(txs, false)
    }

    pub fn add_local(&mut self, tx: Transaction) -> Result<()> {
        self.add_txs(vec![tx], true)
    }

    pub fn add_locals(&mut self, txs: Vec<Transaction>) -> Result<()> {
        self.add_txs(txs, true)
    }
}
