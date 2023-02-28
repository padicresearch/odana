#![feature(btree_drain_filter)]
#![feature(hash_drain_filter)]

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::option::Option::Some;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use primitive_types::address::Address;
use primitive_types::H256;
use tracing::{debug, error, info, trace, warn};
use traits::{Blockchain, StateDB};
use types::block::BlockHeader;
use types::events::LocalEventMessage;
use types::tx::{SignedTransaction, TransactionList, TransactionStatus};
use types::TxPoolConfig;

use crate::error::TxPoolError;
use crate::prque::PriorityQueue;
use crate::tx_list::{NonceTransaction, TxList, TxPricedList, TxSortedList};
use crate::tx_lookup::{AccountSet, TxLookup};
use crate::tx_noncer::TxNoncer;

mod error;
mod prque;
#[cfg(test)]
mod tests;
mod tx_list;
pub mod tx_lookup;
pub mod tx_noncer;

type TransactionRef = Arc<SignedTransaction>;
type Transactions = Vec<TransactionRef>;
const TXPOOL_LOG_TARGET: &str = "txpool";

const TX_SLOT_SIZE: u64 = 32 * 1024;
//const TX_MAX_SIZE: u64 = 4 * TX_SLOT_SIZE;

pub(crate) fn num_slots(tx: &SignedTransaction) -> u64 {
    (tx.size() + TX_SLOT_SIZE - 1) / TX_SLOT_SIZE
}

const DEFAULT_TX_POOL_CONFIG: TxPoolConfig = TxPoolConfig {
    no_locals: false,
    price_ratio: 0.01,
    price_bump: 10,
    account_slots: 16,
    global_slots: 4096,
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

pub struct TxPool {
    config: TxPoolConfig,
    locals: AccountSet,
    chain: Arc<dyn Blockchain>,
    current_state: Arc<dyn StateDB>,
    pending_nonce: TxNoncer,
    //local event emitter
    lmpsc: UnboundedSender<LocalEventMessage>,

    // repacker variables
    queued_events: HashMap<Address, TxSortedList>,
    //end repacker
    pending: HashMap<Address, TxList>,
    queue: HashMap<Address, TxList>,
    beats: HashMap<Address, Instant>,
    all: TxLookup,
    priced: TxPricedList,
    changes_since_repack: i32,
}

impl TxPool {
    pub fn new(
        conf: Option<&TxPoolConfig>,
        local_accounts: Option<Vec<Address>>,
        lmpsc: UnboundedSender<LocalEventMessage>,
        chain: Arc<dyn Blockchain>,
    ) -> Result<Self> {
        let conf = conf.map(sanitize).unwrap_or(DEFAULT_TX_POOL_CONFIG);
        let current_state = chain.get_current_state()?;
        let locals = local_accounts.map(AccountSet::from).unwrap_or_default();
        Ok(Self {
            config: conf,
            locals,
            chain,
            current_state: current_state.clone(),
            pending_nonce: TxNoncer::new(current_state),
            lmpsc,
            queued_events: Default::default(),
            pending: Default::default(),
            queue: Default::default(),
            beats: Default::default(),
            all: TxLookup::new(),
            priced: TxPricedList::new(),
            changes_since_repack: Default::default(),
        })
    }

    fn validate_tx(&self, tx: &SignedTransaction, _local: bool) -> Result<()> {
        let from = tx.sender();
        if self.current_state.nonce(&from) > tx.nonce() {
            return Err(TxPoolError::NonceTooLow.into());
        }
        let sender_balance = self.current_state.balance(&from);
        if sender_balance < tx.fees() + tx.price() {
            return Err(TxPoolError::InsufficientFunds(tx.fees(), tx.price()).into());
        }
        Ok(())
    }

    fn add(&mut self, tx: TransactionRef, local: bool) -> Result<bool> {
        let hash = tx.hash();
        anyhow::ensure!(!self.all.contains(&hash), {
            trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Discarding already known transaction");
            TxPoolError::TransactionAlreadyKnown
        });
        let is_local = local || self.locals.contains_tx(&tx);
        if let Err(e) = self.validate_tx(&tx, is_local) {
            trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), error = ?e, "Discarding invalid transaction");
            anyhow::bail!(e)
        }

        let _num = num_slots(&tx);
        let _all_slots = self.all.slots();
        if self.all.slots() + num_slots(&tx) > self.config.global_slots + self.config.global_queue {
            if !is_local && self.priced.underpriced(tx.clone())? {
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), fee = ?tx.fees(), "Discarding underpriced transaction");
                return Err(TxPoolError::Underpriced.into());
            }
            anyhow::ensure!(
                self.changes_since_repack < (self.config.global_slots / 4) as i32,
                TxPoolError::TxPoolOverflow
            );

            let drop = match self.priced.discard(
                self.all.slots() - (self.config.global_slots + self.config.global_queue)
                    + num_slots(&tx),
            ) {
                Ok(drop) => drop,
                Err(_) => {
                    if !is_local {
                        trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), fee = ?tx.fees(), "Discarding overflown transaction");
                        return Err(TxPoolError::TxPoolOverflow.into());
                    } else {
                        vec![]
                    }
                }
            };

            self.changes_since_repack = drop.len() as i32;
            for tx in drop {
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), fee = ?tx.fees(), "Discarding freshly underpriced transaction");
                self.remove_tx(tx.hash(), false)?;
            }
        }
        let from = tx.sender();
        if let Some(list) = self.pending.get_mut(&from) {
            if list.overlaps(tx.clone()) {
                let (inserted, old) = list.add(tx.clone(), self.config.price_bump);
                anyhow::ensure!(inserted, TxPoolError::ReplaceUnderpriced);
                if let Some(old) = old.clone() {
                    self.all.remove(&old.hash());
                    self.priced.remove(old);
                }

                self.all.add(tx.clone(), is_local);
                self.priced.put(tx.clone(), is_local);
                self.queue_event(tx.clone());
                trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), from = ?from, to = ?tx.to(), "Pooled new executable transaction");
                return Ok(old.is_some());
            }
        }

        let replaced = self.enqueue_tx(hash, tx.clone(), is_local, true)?;
        if local && !self.locals.contains(&from) {
            info!(target : TXPOOL_LOG_TARGET, address = ?from, "Setting new local account");
            self.locals.add(from);
            let migrated = self.all.remote_to_locals(&self.locals);
            for tx in migrated {
                self.priced.remove(tx);
            }
        }
        trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), from = ?from, to = ?tx.to(), "Pooled new future transaction");
        Ok(replaced)
    }

    fn queue_event(&mut self, tx: TransactionRef) {
        let sender = tx.sender();
        let events = self
            .queued_events
            .entry(sender)
            .or_insert_with(Default::default);
        events.put(tx);
    }

    fn enqueue_tx(
        &mut self,
        hash: H256,
        tx: TransactionRef,
        local: bool,
        add_all: bool,
    ) -> Result<bool> {
        let from = tx.sender();
        let queue = self.queue.entry(from).or_insert_with(|| TxList::new(false));
        let (inserted, old) = queue.add(tx.clone(), self.config.price_bump);
        anyhow::ensure!(inserted, TxPoolError::ReplaceUnderpriced);
        if let Some(old) = &old {
            self.all.remove(&old.hash());
            self.priced.remove(old.clone());
        }

        if !self.all.contains(&hash) && !add_all {
            error!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Missing transaction in lookup set");
        }

        if add_all {
            self.all.add(tx.clone(), local);
            self.priced.put(tx, local);
        }

        self.beats.entry(from).or_insert_with(Instant::now);
        Ok(old.is_some())
    }

    fn remove_tx(&mut self, hash: H256, outofbound: bool) -> Result<()> {
        let tx = match self.all.get(&hash) {
            None => {
                return Ok(());
            }
            Some(tx) => tx,
        };

        self.all.remove(&hash);
        let sender = tx.sender();

        if outofbound {
            self.priced.remove(tx.clone());
        }

        if let Some(pending) = self.pending.get_mut(&sender) {
            let (removed, invalids) = pending.remove(tx.clone());
            if removed {
                if pending.is_empty() {
                    self.pending.remove(&sender);
                }

                for tx in invalids {
                    self.enqueue_tx(tx.hash(), tx.clone(), false, false)?;
                }
                self.pending_nonce.set_if_lower(sender, tx.nonce());
                return Ok(());
            }
        }

        if let Some(future) = self.queue.get_mut(&sender) {
            let (removed, _) = future.remove(tx);
            if removed {
                if self.pending.is_empty() {
                    self.queue.remove(&sender);
                }
                return Ok(());
            }
        }
        Ok(())
    }

    fn promote_tx(&mut self, addr: Address, hash: H256, tx: TransactionRef) -> bool {
        let nonce = tx.nonce();
        let list = self
            .pending
            .entry(addr)
            .or_insert_with(|| TxList::new(true));
        let (inserted, old) = list.add(tx.clone(), self.config.price_bump);
        if !inserted {
            // If not inserted an older transaction was better so remove the new transaction completely
            self.all.remove(&hash);
            self.priced.remove(tx);
            return false;
        }
        if let Some(old) = old {
            self.all.remove(&old.hash());
            self.priced.remove(old);
        }
        self.pending_nonce.set(addr, nonce + 1);
        self.beats.insert(addr, Instant::now());
        true
    }

    fn add_txs(&mut self, tsx: Vec<SignedTransaction>, local: bool) -> Result<()> {
        let mut news = Vec::new();
        let mut errors = Vec::with_capacity(tsx.len());
        for tx in tsx.into_iter() {
            if self.all.contains(&tx.hash()) {
                errors.push(format!("{:?}", TxPoolError::TransactionAlreadyKnown));
                continue;
            }
            news.push(Arc::new(tx))
        }

        if news.is_empty() {
            return Err(TxPoolError::CompositeErrors(errors).into());
        }

        let (dirty, new_errors) = self.add_txs_locked(news, local);
        for err in new_errors {
            errors.push(err);
        }
        self.repack(dirty, None)?;
        self.queued_events = HashMap::new();

        if !errors.is_empty() {
            return Err(TxPoolError::CompositeErrors(errors).into());
        }
        Ok(())
    }

    fn add_txs_locked(
        &mut self,
        tsx: Vec<TransactionRef>,
        local: bool,
    ) -> (AccountSet, Vec<String>) {
        let mut dirty = AccountSet::new();
        let mut errors = Vec::with_capacity(tsx.len());
        for (i, tx) in tsx.into_iter().enumerate() {
            match self.add(tx.clone(), local) {
                Ok(replaced) => {
                    if !replaced {
                        dirty.add_tx(tx);
                    }
                }
                Err(error) => {
                    errors.insert(i, format!("{}", error));
                }
            };
        }
        (dirty, errors)
    }

    #[allow(unused_assignments)]
    fn reset(&mut self, old_head: Option<BlockHeader>, new_head: BlockHeader) -> Result<()> {
        let mut reinject = Vec::new();
        if let Some(old_head) = old_head {
            if old_head.hash().ne(&new_head.parent_hash) {
                let old_num = old_head.level;
                let new_num = new_head.level;
                let depth = (old_num as i32 - new_num as i32).abs();
                if depth > 64 {
                    debug!(target : TXPOOL_LOG_TARGET, depth = ?depth, "Skipping deep transaction repack");
                } else {
                    let mut discarded = BTreeSet::new();
                    let mut included = BTreeSet::new();
                    let rem = self.chain.get_block(&old_head.hash(), old_head.level)?;
                    let mut add = match self.chain.get_block(&new_head.hash(), new_head.level)? {
                        None => {
                            error!(target : TXPOOL_LOG_TARGET, new_head = ?new_head.hash(), "Transaction pool reset with missing newhead");
                            return Err(TxPoolError::MissingBlock.into());
                        }
                        Some(add) => add,
                    };

                    if let Some(mut rem) = rem {
                        while rem.level() > add.level() {
                            discarded.extend(
                                rem.transactions()
                                    .iter()
                                    .map(|tx| NonceTransaction(Arc::new(tx.clone()))),
                            );
                            if let Some(block) =
                                self.chain.get_block(rem.parent_hash(), rem.level() - 1)?
                            {
                                rem = block;
                            } else {
                                error!(target : TXPOOL_LOG_TARGET,block = ?old_head.hash(),level = ?old_num,"Unrooted old chain seen by tx pool");
                                return Ok(());
                            }
                        }
                        while add.level() > rem.level() {
                            included.extend(
                                add.transactions()
                                    .iter()
                                    .map(|tx| NonceTransaction(Arc::new(tx.clone()))),
                            );
                            if let Some(block) =
                                self.chain.get_block(add.parent_hash(), add.level() - 1)?
                            {
                                add = block;
                            } else {
                                error!(target : TXPOOL_LOG_TARGET,block = ?new_head.hash(),level = ?new_num,"Unrooted new chain seen by tx pool");
                                return Ok(());
                            }
                        }
                        while rem.hash() != add.hash() {
                            discarded.extend(
                                rem.transactions()
                                    .iter()
                                    .map(|tx| NonceTransaction(Arc::new(tx.clone()))),
                            );
                            if let Some(block) =
                                self.chain.get_block(rem.parent_hash(), rem.level() - 1)?
                            {
                                rem = block;
                            } else {
                                error!(target : TXPOOL_LOG_TARGET,block = ?old_head.hash(),level = ?old_num,"Unrooted old chain seen by tx pool");
                                return Ok(());
                            }
                            included.extend(
                                add.transactions()
                                    .iter()
                                    .map(|tx| NonceTransaction(Arc::new(tx.clone()))),
                            );
                            if let Some(block) =
                                self.chain.get_block(add.parent_hash(), add.level() - 1)?
                            {
                                add = block;
                            } else {
                                error!(target : TXPOOL_LOG_TARGET,block = ?new_head.hash(),level = ?new_num,"Unrooted new chain seen by tx pool");
                                return Ok(());
                            }
                        }
                        reinject = discarded
                            .intersection(&included)
                            .map(|tx| tx.0.clone())
                            .collect();
                    } else {
                        if new_num > old_num {
                            warn!(target : TXPOOL_LOG_TARGET,
                            old = ?old_head.hash(),
                            old_level = ?old_num,
                            new = ?new_head.hash(),
                            new_level = ?new_num,
                            "Transaction pool reset with missing newhead");
                            return Ok(());
                        }
                        debug!(target : TXPOOL_LOG_TARGET,
                            old = ?old_head.hash(),
                            old_level = ?old_num,
                            new = ?new_head.hash(),
                            new_level = ?new_num,
                            "Skipping transaction reset caused by setHead");
                    }
                }
            }
        }
        let state = match self.chain.get_state_at(&new_head.state_root) {
            Ok(state) => state,
            Err(e) => {
                error!(target : TXPOOL_LOG_TARGET, error = ?e, "Failed to reset txpool state");
                return Err(e);
            }
        };
        self.current_state = state.clone();
        self.pending_nonce = TxNoncer::new(state);
        debug!(target : TXPOOL_LOG_TARGET, count = ?reinject.len(), "Reinjecting stale transactions");
        self.add_txs_locked(reinject, false);
        Ok(())
    }

    fn truncate_pending(&mut self) {
        let mut pending = self
            .pending
            .iter()
            .fold(0, |acc, (_, list)| acc + list.len()) as u64;
        if pending <= self.config.global_slots {
            return;
        }

        let mut spammers = PriorityQueue::new();
        for (addr, list) in self.pending.iter() {
            if !self.locals.contains(addr) && list.len() as u64 > self.config.account_slots {
                spammers.push(*addr, list.len() as i64);
            }
        }

        let mut offenders = Vec::new();
        while pending > self.config.global_slots && !spammers.is_empty() {
            let offender = if let Some((offender, _)) = spammers.pop() {
                offender
            } else {
                return;
            };
            offenders.push(offender);
            if offenders.len() > 1 {
                let threshold = if let Some(list) = self.pending.get(&offender) {
                    list.len()
                } else {
                    0
                };

                while pending > self.config.global_slots
                    && self
                        .pending
                        .get(&offenders[offenders.len() - 2])
                        .map(|tx| tx.len())
                        .unwrap_or_default()
                        > threshold
                {
                    for offender in offenders.iter() {
                        let list = match self.pending.get_mut(offender) {
                            None => return,
                            Some(list) => list,
                        };
                        let caps = list.cap(list.len().saturating_sub(1) as u64);
                        for tx in caps {
                            let hash = tx.hash();
                            let nonce = tx.nonce();
                            self.all.remove(&hash);
                            self.priced.remove(tx);
                            self.pending_nonce.set_if_lower(*offender, nonce);
                            trace!(target : TXPOOL_LOG_TARGET, hash = ?hash, "Removed fairness-exceeding pending transaction");
                        }
                        pending -= 1;
                    }
                }
            }
        }
        // If still above threshold, reduce to limit or min allowance
        if pending > self.config.global_slots && !offenders.is_empty() {
            while pending > self.config.global_slots
                && self
                    .pending
                    .get(&offenders[offenders.len() - 1])
                    .map(|tx| tx.len() as u64)
                    .unwrap_or_default()
                    > self.config.account_slots
            {
                for addr in offenders.iter() {
                    if let Some(list) = self.pending.get_mut(addr) {
                        let caps = list.cap(list.len().saturating_sub(1) as u64);
                        for tx in caps {
                            let hash = tx.hash();
                            let nonce = tx.nonce();
                            let hash_256 = tx.hash();
                            self.all.remove(&hash);
                            self.priced.remove(tx);
                            self.pending_nonce.set_if_lower(*addr, nonce);
                            trace!(target : TXPOOL_LOG_TARGET, hash = ?hash_256, "Removed fairness-exceeding pending transaction");
                        }
                        pending -= 1;
                    }
                }
            }
        }
    }

    fn truncate_queue(&mut self) -> Result<()> {
        let queued = self.queue.iter().fold(0, |_acc, (_, list)| list.len()) as u64;
        if queued <= self.config.global_queue {
            return Ok(());
        }

        let mut addresses = BTreeSet::new();
        for (addr, _) in self.queue.iter() {
            if !self.locals.contains(addr) {
                addresses.insert(AddressByHeartbeat::new(
                    *addr,
                    *self.beats.get(addr).unwrap(),
                ));
            }
        }
        let mut addresses: Vec<_> = addresses.iter().copied().collect();
        let mut drop = queued - self.config.global_queue;
        while drop > 0 && !addresses.is_empty() {
            let addr = addresses[addresses.len() - 1];
            let list = match self.queue.get(&addr.address) {
                None => {
                    return Ok(());
                }
                Some(list) => list,
            };
            addresses = addresses[..addresses.len() - 1].to_owned();
            let size = list.len() as u64;
            if size <= drop {
                for tx in list.flatten() {
                    self.remove_tx(tx.hash(), true)?;
                }
                drop -= size;
                continue;
            }
            let txs = list.flatten();
            let mut i = txs.len() as isize;
            while i >= 0 && drop > 0 {
                self.remove_tx(txs[i as usize].hash(), true)?;
                drop -= 1;
                i -= 1;
            }
        }
        Ok(())
    }

    fn demote_unexecutable(&mut self) -> Result<()> {
        let mut stale_addrs = Vec::new();
        let mut enqueue = Vec::new();
        for (addr, list) in self.pending.iter_mut() {
            let nonce = self.current_state.nonce(addr);
            let olds = list.forward(nonce);
            for tx in olds {
                let hash = tx.hash();
                self.all.remove(&hash);
                trace!(target : TXPOOL_LOG_TARGET, hash = ?hash, "Removed old pending transaction");
            }
            if let Some((drops, invlaids)) = list.filter(self.current_state.balance(addr)) {
                for tx in drops {
                    let hash = tx.hash();
                    trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Removed unpayable pending transaction");
                    self.all.remove(&hash);
                }
                for tx in invlaids {
                    trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Demoting pending transaction");
                    enqueue.push(tx)
                }
            }

            if !list.is_empty() && list.txs.get(nonce).is_none() {
                let gapped = list.cap(0);
                for tx in gapped {
                    trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Demoting invalidated transaction");
                    enqueue.push(tx)
                }
            }
        }
        for tx in enqueue {
            let hash = tx.hash();
            self.enqueue_tx(hash, tx, false, false)?;
        }

        for (addr, list) in self.pending.iter() {
            if list.is_empty() {
                stale_addrs.push(*addr)
            }
        }

        for addr in stale_addrs {
            self.pending.remove(&addr);
        }

        Ok(())
    }

    fn promote_executable(&mut self, accounts: Vec<Address>) -> Vec<TransactionRef> {
        let mut promoted = Vec::new();

        for addr in &accounts {
            let list = match self.queue.get_mut(addr) {
                None => {
                    continue;
                }
                Some(list) => list,
            };

            let forward = list.forward(self.current_state.nonce(addr));
            for tx in forward.iter() {
                self.all.remove(&tx.hash());
            }
            trace!(target : TXPOOL_LOG_TARGET, count = ?forward.len(), "Removed old queued transactions");

            let drops = list
                .filter(self.current_state.balance(addr))
                .map(|(drops, _)| drops)
                .unwrap_or_default();
            for tx in drops.iter() {
                self.all.remove(&tx.hash());
            }
            trace!(target : TXPOOL_LOG_TARGET, count = ?drops.len(), "Removed unpayable queued transactions");

            let readies = list.ready(self.pending_nonce.get(addr));
            let ready_count = &readies.len();
            for tx in readies {
                let hash = tx.hash();
                if self.promote_tx(*addr, hash, tx.clone()) {
                    promoted.push(tx)
                }
            }
            trace!(target : TXPOOL_LOG_TARGET, count = ?ready_count, "Promoted queued transactions");
        }

        for addr in &accounts {
            let list = match self.queue.get_mut(addr) {
                None => {
                    continue;
                }
                Some(list) => list,
            };
            if !self.locals.contains(addr) {
                let caps = list.cap(self.config.account_queue);
                for tx in caps {
                    let hash = tx.hash();
                    self.all.remove(&hash);
                    trace!(target : TXPOOL_LOG_TARGET, hash = ?tx.hash(), "Removed cap-exceeding queued transaction");
                }
            }

            if list.is_empty() {
                self.queue.remove(addr);
                self.beats.remove(addr);
            }
        }

        promoted
    }

    pub fn repack(
        &mut self,
        dirty_accounts: AccountSet,
        reset: Option<ResetRequest>,
    ) -> Result<()> {
        let mut events = self.queued_events.clone();
        let mut promote_addrs = Vec::new();
        if !dirty_accounts.is_empty() && reset.is_none() {
            promote_addrs = dirty_accounts.flatten();
        }

        if let Some(reset) = &reset {
            self.reset(reset.old_head, reset.new_head)?;
            for (addr, list) in events.iter_mut() {
                list.forward(self.pending_nonce.get(addr));
            }
            let _ = events.drain_filter(|_, tx| tx.is_empty());
            promote_addrs = Vec::new();
            for (addr, _) in self.queue.iter() {
                promote_addrs.push(*addr)
            }
        }
        let promoted = self.promote_executable(promote_addrs);
        if reset.is_some() {
            self.demote_unexecutable()?;
            let mut nonces = HashMap::with_capacity(self.pending.len());
            for (addr, list) in self.pending.iter_mut() {
                if let Some(highest_pending) = list.last_element() {
                    nonces.insert(*addr, highest_pending.nonce() + 1);
                }
                self.pending_nonce.set_all(&nonces);
            }
        }
        self.truncate_pending();
        self.truncate_queue()?;

        self.changes_since_repack = 0;

        for tx in promoted {
            let sorted_map = events.entry(tx.sender()).or_insert_with(TxSortedList::new);
            sorted_map.put(tx);
        }
        let mut txs = Vec::new();
        if !events.is_empty() {
            for (_, set) in events {
                txs.extend(set.flatten())
            }
            self.send(txs.clone())?;
        }
        Ok(())
    }

    fn send(&self, txs: Vec<TransactionRef>) -> Result<()> {
        use std::ops::Deref;
        self.lmpsc
            .send(LocalEventMessage::TxPoolPack(
                txs.into_iter().map(|tx| tx.deref().clone()).collect(),
            ))
            .map_err(|e| e.into())
    }
}

//Public functions
impl TxPool {
    pub fn add_local(&mut self, tx: SignedTransaction) -> Result<()> {
        self.add_txs(vec![tx], true)
    }

    pub fn add_locals(&mut self, txs: Vec<SignedTransaction>) -> Result<()> {
        self.add_txs(txs, true)
    }

    pub fn add_remote(&mut self, tx: SignedTransaction) -> Result<()> {
        self.add_txs(vec![tx], false)
    }

    pub fn add_remotes(&mut self, txs: Vec<SignedTransaction>) -> Result<()> {
        self.add_txs(txs, false)
    }

    pub fn get(&self, hash: &H256) -> Option<TransactionRef> {
        self.all.get(hash)
    }

    pub fn has(&self, hash: &H256) -> bool {
        self.all.contains(hash)
    }

    pub fn status(&self, txs: Vec<H256>) -> Vec<TransactionStatus> {
        let mut status = vec![TransactionStatus::NotFound; txs.len()];
        for (i, hash) in txs.iter().enumerate() {
            if let Some(tx) = self.get(hash) {
                let sender = tx.sender();
                if let Some(list) = self.pending.get(&sender) {
                    status[i] = if list.txs.has(tx.nonce()) {
                        TransactionStatus::Pending
                    } else {
                        TransactionStatus::NotFound
                    }
                } else if let Some(list) = self.queue.get(&sender) {
                    status[i] = if list.txs.has(tx.nonce()) {
                        TransactionStatus::Pending
                    } else {
                        TransactionStatus::NotFound
                    }
                }
            }
        }
        status
    }

    pub fn nonce(&self, address: &Address) -> u64 {
        let mut nonce = self.current_state.nonce(address);
        let sn = self.pending_nonce.get(address);
        if sn > nonce {
            nonce = sn
        }
        if sn < nonce {
            self.pending_nonce.set(*address, nonce);
        }
        nonce
    }

    pub fn stats(&self) -> (usize, usize) {
        let mut pending = 0;
        for (_, list) in self.pending.iter() {
            pending += list.len()
        }
        let mut queued = 0;
        for (_, list) in self.queue.iter() {
            queued += list.len()
        }
        (pending, queued)
    }

    pub fn content(
        &self,
    ) -> (
        HashMap<Address, TransactionList>,
        HashMap<Address, TransactionList>,
    ) {
        let mut pending = HashMap::new();
        for (address, list) in self.pending.iter() {
            pending.insert(*address, TransactionList::new(list.flatten()));
        }

        let mut queued = HashMap::new();
        for (address, list) in self.queue.iter() {
            queued.insert(*address, TransactionList::new(list.flatten()));
        }
        (pending, queued)
    }

    pub fn content_from(&self, address: &Address) -> (Transactions, Transactions) {
        let mut pending = Vec::new();
        if let Some(list) = self.pending.get(address) {
            pending = list.flatten();
        }
        let mut queued = Vec::new();
        if let Some(list) = self.queue.get(address) {
            queued = list.flatten();
        }
        (pending, queued)
    }

    pub fn pending(&self) -> HashMap<Address, TransactionList> {
        let mut pending = HashMap::new();
        for (address, list) in self.pending.iter() {
            pending.insert(*address, TransactionList::new(list.flatten()));
        }
        pending
    }

    pub fn locals(&self) -> Vec<Address> {
        self.locals.flatten()
    }
}

#[derive(Debug)]
pub struct ResetRequest {
    old_head: Option<BlockHeader>,
    new_head: BlockHeader,
}

impl ResetRequest {
    pub fn new(old: Option<BlockHeader>, new: BlockHeader) -> Self {
        Self {
            old_head: old,
            new_head: new,
        }
    }
}

#[derive(Copy, Clone)]
struct AddressByHeartbeat {
    address: Address,
    heartbeat: Instant,
}

impl AddressByHeartbeat {
    fn new(address: Address, heartbeat: Instant) -> Self {
        Self { address, heartbeat }
    }
}

impl PartialOrd for AddressByHeartbeat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.heartbeat.partial_cmp(&other.heartbeat)
    }
}

impl PartialEq for AddressByHeartbeat {
    fn eq(&self, other: &Self) -> bool {
        self.address.eq(&other.address)
    }
}

impl Eq for AddressByHeartbeat {}

impl Ord for AddressByHeartbeat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.heartbeat.cmp(&other.heartbeat)
    }
}
