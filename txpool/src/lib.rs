#![feature(map_first_last)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::{Error, Result};
use dashmap::DashMap;

use account::GOVERNANCE_ACCOUNTID;
use primitive_types::H160;
use tracing::{info, trace, warn};
use traits::{ChainState, StateDB};
use types::{TxHash, TxPoolConfig};
use types::tx::{Transaction, TransactionKind};

use crate::error::TxPoolError;
use crate::tx_list::{TxList, TxPricedList};
use crate::tx_lookup::{AccountSet, TxLookup};
use crate::tx_noncer::TxNoncer;

mod tests;
mod tx_list;
mod tx_lookup;
mod tx_noncer;
mod error;

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
    life_time: Duration::from_secs(3 * 3600)
};

fn sanitize(conf : &TxPoolConfig) -> TxPoolConfig {
    let default = DEFAULT_TX_POOL_CONFIG;
    let mut conf = *conf;
    //Todo : Variable transaction fees
    if conf.price_ratio != 0.01 {
        warn!(target : TXPOOL_LOG_TARGET, provided = conf.price_ratio, updated = default.price_ratio, "Sanitizing invalid txpool price ratio");
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

pub struct TxPool<Chain> where Chain: ChainState {
    mu: RwLock<()>,
    locals: AccountSet,
    chain: Chain,
    current_state: Arc<dyn StateDB>,
    pending_nonce: TxNoncer,

    pending: DashMap<Address, TxList>,
    queue: DashMap<Address, TxList>,
    beats: DashMap<Address, Instant>,
    all: TxLookup,
    priced: TxPricedList,
}

impl<Chain> TxPool<Chain> where Chain: ChainState {
    pub fn new(chain: Chain) -> Result<Self> {
        let current_state = chain.get_current_state()?;
        Ok(Self {
            mu: Default::default(),
            locals: AccountSet::new(),
            chain,
            current_state: current_state.clone(),
            pending_nonce: TxNoncer::new(current_state),
            pending: Default::default(),
            queue: Default::default(),
            beats: Default::default(),
            all: TxLookup::new(),
            priced: TxPricedList::new(),
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

    pub fn content(&self) -> (HashMap<Address, Transactions>, HashMap<Address, Transactions>) {
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
            TransactionKind::Coinbase {
                ..
            } => {
                anyhow::bail!(TxPoolError::ExplictCoinbase);
            }
        }
        let from = tx.sender_address();
        anyhow::ensure!(self.current_state.nonce(&from) < tx.nonce(), TxPoolError::NonceTooLow);
        anyhow::ensure!(self.current_state.balance(&from) > tx.price(), TxPoolError::InsufficientFunds);

        account::verify_signature(
            tx.origin(),
            tx.signature(),
            &tx.sig_hash()?,
        )
    }

    fn add(&self, tx: Transaction, local: bool) -> Result<bool> {
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


        Ok(false)
    }
}
