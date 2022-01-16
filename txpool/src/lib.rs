#![feature(map_first_last)]
#![feature(btree_drain_filter)]

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicI32;
use std::time::{Duration, Instant};

use anyhow::{Error, Result};
use dashmap::DashMap;

use account::GOVERNANCE_ACCOUNTID;
use primitive_types::H160;
use tracing::{error, info, trace, warn};
use traits::{ChainState, StateDB};
use types::{Hash, TxHash, TxPoolConfig};
use types::tx::{Transaction, TransactionKind};

use crate::error::TxPoolError;
use crate::tx_list::{NonceTransaction, TxList, TxPricedList};
use crate::tx_lookup::{AccountSet, TxLookup};
use crate::tx_noncer::TxNoncer;

mod error;
mod tests;
mod tx_list;
mod tx_lookup;
mod tx_noncer;

type TxHashRef = Arc<TxHash>;
type TransactionRef = Arc<Transaction>;
type Transactions = Vec<TransactionRef>;
type Address = H160;

const TXPOOL_LOG_TARGET: &str = "txpool";

const TX_SLOT_SIZE: u64 = 32 * 1024;
const TX_MAX_SIZE: u64 = 4 * TX_SLOT_SIZE;

pub(crate) fn num_slots(tx: &Transaction) -> u64 {
    return (tx.size() + TX_SLOT_SIZE - 1) / TX_SLOT_SIZE;
}

const DEFAULT_TX_POOL_CONFIG: TxPoolConfig = TxPoolConfig {
    no_locals: false,
    price_ratio: 0.01,
    price_bump: 10,
    account_slots: 16,
    global_slots: 4096 + 1024,
    account_queue: 64,
    global_queue: 1024,
    life_time: Duration::from_secs(3 * 3600),
};

fn sanitize(conf: &TxPoolConfig) -> TxPoolConfig {
    let default = DEFAULT_TX_POOL_CONFIG;
    let mut conf = *conf;
    //Todo : Variable transaction fees
    if conf.price_ratio != 0.01 {
        warn!(
            target: TXPOOL_LOG_TARGET,
            provided = conf.price_ratio,
            updated = default.price_ratio,
            "Sanitizing invalid txpool price ratio"
        );
        conf.price_ratio = default.price_ratio
    }
    if conf.price_bump < 1 {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.price_bump, updated = ?default.price_bump, "Sanitizing invalid txpool price bump");
        conf.price_bump = default.price_bump
    }
    if conf.account_slots < 1 {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.account_slots, updated = ?default.account_slots, "Sanitizing invalid txpool account slots");
        conf.account_slots = default.account_slots
    }
    if conf.global_slots < 1 {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.global_slots, updated = ?default.global_slots, "Sanitizing invalid txpool global slots");
        conf.global_slots = default.global_slots
    }

    if conf.account_queue < 1 {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.account_queue, updated = ?default.account_queue, "Sanitizing invalid txpool account queue");
        conf.account_queue = default.account_queue
    }

    if conf.global_queue < 1 {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.global_queue, updated = ?default.global_queue, "Sanitizing invalid txpool global queue");
        conf.global_queue = default.global_queue
    }

    if conf.life_time < Duration::from_secs(1) {
        warn!(target : TXPOOL_LOG_TARGET, provided = ?conf.life_time, updated = ?default.life_time, "Sanitizing invalid txpool life time");
        conf.life_time = default.life_time
    }

    conf
}

pub struct TxPool<Chain>
    where
        Chain: ChainState,
{
    mu: RwLock<()>,
    config: TxPoolConfig,
    locals: AccountSet,
    chain: Chain,
    current_state: Arc<dyn StateDB>,
    pending_nonce: TxNoncer,

    queued_events : DashMap<Address, BTreeSet<NonceTransaction>>,

    pending: DashMap<Address, TxList>,
    queue: DashMap<Address, TxList>,
    beats: DashMap<Address, Instant>,
    all: TxLookup,
    priced: TxPricedList,
    changes_since_repack: AtomicI32,
}

impl<Chain> TxPool<Chain>
    where
        Chain: ChainState,
{
    pub fn new(
        conf: Option<&TxPoolConfig>,
        local_accounts: Option<Vec<Address>>,
        chain: Chain,
    ) -> Result<Self> {
        let conf = conf
            .map(|conf| sanitize(conf))
            .unwrap_or(DEFAULT_TX_POOL_CONFIG);
        let current_state = chain.get_current_state()?;
        let locals = local_accounts
            .map(|locals| AccountSet::from(locals))
            .unwrap_or(AccountSet::new());
        Ok(Self {
            mu: Default::default(),
            config: conf,
            locals,
            chain,
            current_state: current_state.clone(),
            pending_nonce: TxNoncer::new(current_state),
            queued_events: Default::default(),
            pending: Default::default(),
            queue: Default::default(),
            beats: Default::default(),
            all: TxLookup::new(),
            priced: TxPricedList::new(),
            changes_since_repack: Default::default()
        })
    }

    pub fn nonce(&self, address: &H160) -> u64 {
        self.pending_nonce.get(address)
    }

    pub fn stats(&self) -> (usize, usize) {
        let mut pending = 0;
        for (_, list) in self.pending.iter().map(|re| (re.key(), re.value())) {
            pending += list.len()
        }
        let mut queued = 0;
        for (_, list) in self.queue.iter().map(|re| (re.key(), re.value())) {
            queued += list.len()
        }
        (pending, queued)
    }

    pub fn content(
        &self,
    ) -> (
        HashMap<Address, Transactions>,
        HashMap<Address, Transactions>,
    ) {
        let mut pending = HashMap::new();
        for (address, list) in self.pending.iter().map(|re| (re.key(), re.value())) {
            pending.insert(*address, list.flatten());
        }

        let mut queued = HashMap::new();
        for (address, list) in self.queue.iter().map(|re| (re.key(), re.value())) {
            queued.insert(*address, list.flatten());
        }
        (pending, queued)
    }

    pub fn content_from(&self, address: &Address) -> (Transactions, Transactions) {
        let mut pending = Vec::new();
        if let Some(list) = self.pending.get(address).map(|r| r.value()) {
            pending = list.flatten();
        }
        let mut queued = Vec::new();
        if let Some(list) = self.queue.get(address).map(|r| r.value()) {
            queued = list.flatten();
        }
        (pending, queued)
    }

    pub fn pending(&self) -> HashMap<Address, Transactions> {
        let mut pending = HashMap::new();
        for (address, list) in self.pending.iter().map(|re| (re.key(), re.value())) {
            pending.insert(*address, list.flatten());
        }
        (pending)
    }

    pub fn locals(&self) -> Vec<Address> {
        self.locals.flatten().unwrap_or_default()
    }

    fn validate_tx(&self, tx: &Transaction, local: bool) -> Result<()> {
        match tx.kind() {
            TransactionKind::Transfer { from, .. } => {
                if from != tx.origin() && tx.origin() != &GOVERNANCE_ACCOUNTID {
                    anyhow::bail!(TxPoolError::BadOrigin)
                }
            }
            TransactionKind::Coinbase { .. } => {
                anyhow::bail!(TxPoolError::ExplictCoinbase);
            }
        }
        let from = tx.sender();
        anyhow::ensure!(
            self.current_state.nonce(&from) < tx.nonce(),
            TxPoolError::NonceTooLow
        );
        anyhow::ensure!(
            self.current_state.balance(&from) > tx.price(),
            TxPoolError::InsufficientFunds
        );

        account::verify_signature(tx.origin(), tx.signature(), &tx.sig_hash()?)
    }

    fn add(&self, tx: TransactionRef, local: bool) -> Result<bool> {
        let hash = tx.hash();
        anyhow::ensure!(!self.all.contains(&hash)?, {
            trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), "Discarding already known transaction");
            TxPoolError::TransactionAlreadyKnown
        });
        let is_local = local || self.locals.contains_tx(&tx)?;
        match self.validate_tx(&tx, is_local) {
            Err(e) => {
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), error = ?e, "Discarding invalid transaction");
                anyhow::bail!(e)
            }
            _ => {}
        }
        let tx = Arc::new(tx);

        // If the transaction pool is full, discard underpriced transactions
        if self.all.slots() + num_slots(&tx) > self.config.global_slots + self.config.global_queue {
            if !is_local && self.priced.underpriced(*tx)? {
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), fee = ?tx.fees(), "Discarding underpriced transaction");
                return Err(TxPoolError::Underpriced.into());
            }
            let changes_since_repack = self.changes_since_repack.load(std::sync::atomic::Ordering::Relaxed);
            anyhow::ensure!( changes_since_repack < (self.config.global_slots / 4 as i32), TxPoolError::TxPoolOverflow);

            let drop = match self.priced.discard(self.all.slots() - (self.config.global_slots +  self.config.global_queue) + num_slots(&tx)) {
                Ok(drop) => {
                    drop
                }
                Err(_) => {
                    if !is_local  {
                        trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), fee = ?tx.fees(), "Discarding overflown transaction");
                        return Err(TxPoolError::TxPoolOverflow.into())
                    }else {
                        0
                    }
                }
            };

            self.changes_since_repack.fetch_add(drop.len() as i32, std::sync::atomic::Ordering::Relaxed);
            for tx in drop {
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), fee = ?tx.fees(), "Discarding freshly underpriced transaction");
                self.remove_tx(tx.hash(), false)?;
            }
        }
        let from = tx.sender();
        if let Some(list) = self.pending.get_mut(&from).map(|mut r| r.value_mut()) {
            let (inserted,old) =  list.add(*tx, self.config.price_bump);
            anyhow::ensure!(inserted, TxPoolError::ReplaceUnderpriced);
            if let Some(old) = old {
                self.all.remove(&old.hash())?;
                self.priced.remove(old)?;
            }

            self.all.add(tx.clone(), is_local)?;
            self.priced.add(tx.clone(), is_local)?;
            self.queued_events(tx.clone());
            trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), from = ?from, to = ?tx.to(), "Pooled new executable transaction");

        }

        let replaced = self.enqueue_tx(hash, *tx, is_local, true)?;
        if local && !self.locals.contains(&from)? {
            info!(target : TXPOOL_LOG_TARGET, address = ?from, "Setting new local account");
            self.locals.add(from)?;
            let migrated = self.all.remote_to_locals(&self.locals)?;
            for tx in migrated {
                self.priced.remove(tx)
            }
        }
        trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), from = ?from, to = ?tx.to(), "Pooled new future transaction");
        Ok(replaced)
    }

    fn queue_event(&self, tx : TransactionRef) {
        let sender = tx.sender();
        let mut events = self.queued_events.entry(sender).or_insert(Default::default()).value_mut();
        events.insert(NonceTransaction(tx));
    }

    fn enqueue_tx(&self, hash : Hash, tx : TransactionRef, local : bool, add_all : bool) -> Result<bool> {
        let from = tx.sender();
        let queue = self.queue.entry(from).or_insert(TxList::new(false)).value_mut();
        let (inserted,old) = queue.add(tx.clone(), self.config.price_bump);
        anyhow::ensure!(inserted, TxPoolError::ReplaceUnderpriced);
        if let Some(old) = old {
            self.all.remove(&old.hash())?;
            self.priced.remove(old)?;
        }

        if !self.all.contains(&hash)? && !add_all{
            error!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash_256(), "Missing transaction in lookup set");
        }

        if add_all {
            self.all.add(tx.clone(), local)?;
            self.priced.put(tx, local)?;
        }

        if !self.beats.contains_key(&from) {
            self.beats.insert(from, Instant::now())
        }
        Ok(old.is_some())
    }

    fn remove_tx(&self, hash : Hash, outofbound : bool) -> Result<()> {
        let tx = match self.all.get(&hash)? {
            None => {
                return Ok(())
            }
            Some(tx) => {
                tx
            }
        };

        self.all.remove(&hash)?;
        let sender = tx.sender();

        if outofbound {
            self.priced.remove(tx.clone())?;
        }

        if let Some(pending) = self.pending.get_mut(&sender).map(|mut r|r.value_mut()) {
            let (removed, invalids) = pending.remove(tx.clone());
            if removed {
                if pending.is_empty() {
                    self.pending.remove(&sender)
                }

                for tx in invalids {
                    self.enqueue_tx(tx.hash(), tx.clone(), false, false)?;
                }
                self.pending_nonce.set_if_lower(sender, tx.nonce());
                return Ok(())
            }

        }

        if let Some(future) = self.queue.get_mut(&sender).map(|mut r|r.value_mut()) {
            let (removed, _) = future.remove(tx);
            if removed {
                if pending.is_empty() {
                    self.queue.remove(&sender)
                }
                return Ok(())
            }

        }
        Ok(())
    }

    fn promote_tx(&self, addr : H160, hash : Hash, tx : TransactionRef) -> Result<bool> {
        let mut list = self.pending.entry(addr).or_insert(TxList::new(true)).value_mut();
        let (inserted, old) = list.add(tx, self.config.price_bump);
        if !inserted {
            self.all.add(tx.clone(), local)?;
            self.priced.put(tx.clone(), local)?;
            return Ok(false)
        }
        self.pending_nonce.set(addr, tx.nonce() + 1);
        self.beats.insert(addr, Instant::now());
        Ok(true)
    }

    fn add_txs(&self, tsx : Vec<Transaction>, local : bool) ->  Vec<Option<Error>>{

        let mut news = Vec::new();
        let mut errors = vec![None; tsx.len()];
        for (i, tx) in tsx.into_iter().enumerate() {
            if self.all.contains(&tx.hash()) {
                errors[i] = Some(TxPoolError::TransactionAlreadyKnown.into());
                continue
            }
            news.push(Arc::new(tx))
        }

        if news.is_empty() {
            return errors
        }

        self.mu.write().unwrap();

        let (dirty, new_errors) = self.add_txs_locked(news, local);
        let mut none_slot : usize = 0;
        for err in new_errors {
            while errors[none_slot].is_some() {
                none_slot += 1;
            }

            errors[none_slot] = Some(err);
        }

        //Promote executable (SYNC)

        return errors
    }

    fn add_txs_locked(&self, tsx : Vec<TransactionRef>, local : bool) -> (AccountSet,Vec<anyhow::Error>){

        let dirty = AccountSet::new();
        let mut errors = Vec::with_capacity(tsx.len());
        for (i, tx) in tsx.into_iter().enumerate() {
            match self.add(tx.clone(), local) {
                Ok(replaced) => {
                    if !replaced {
                        match dirty.add_tx(tx) {
                           Err(e) => {
                               errors.insert(i, e);
                           }
                            _ => {}
                        }
                    }
                }
                Err(error) => {
                    errors.insert(i, error);
                }
            };
        }
        (dirty, errors)
    }
}


fn schedule_repack_loop<C>(txpool : Arc<TxPool<C>>) where C : ChainState {
    let mut
}