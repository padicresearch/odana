use std::collections::{BTreeMap, BTreeSet};
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize};

use anyhow::Result;

use primitive_types::H160;
use types::Hash;
use types::tx::Transaction;

use crate::{Address, num_slots, TransactionRef, Transactions};

pub struct AccountSet {
    accounts: BTreeSet<H160>,
}

impl From<Vec<Address>> for AccountSet {
    fn from(addresses: Vec<Address>) -> Self {
        let accounts = BTreeSet::from_iter(addresses.into_iter());
        Self {
            accounts
        }
    }
}

impl AccountSet {
    pub(crate) fn new() -> Self {
        Self {
            accounts: Default::default()
        }
    }

    pub(crate) fn contains(&self, address: &H160) -> bool {
        self.accounts.contains(address)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub(crate) fn contains_tx(&self, tx: &Transaction) -> bool {
        let address = tx.sender();
        self.contains(&address)
    }

    pub(crate) fn add(&mut self, address: H160){
        self.accounts.insert(address);
    }

    pub(crate) fn add_tx(&mut self, tx: TransactionRef){
        let address = tx.sender();
        self.add(address);
    }

    pub(crate) fn flatten(&self) -> Vec<H160> {
        self.accounts.iter().map(|addrs| addrs.clone()).collect()
    }

    pub(crate) fn merge(&mut self, other : &AccountSet){
        self.accounts.extend(other.accounts.iter());
    }
}

pub struct TxLookup {
    slots: u64,
    locals: BTreeMap<Hash, TransactionRef>,
    remotes: BTreeMap<Hash, TransactionRef>,
}

impl TxLookup {
    pub fn new() -> Self {
        Self {
            slots: Default::default(),
            locals: Default::default(),
            remotes: Default::default(),
        }
    }
}

impl TxLookup {
    pub fn range(
        &self,
        f: fn(&Hash, &TransactionRef, bool) -> bool,
        local: bool,
        remote: bool,
    ) {
        if local {
            for (key, value) in self.locals.iter() {
                if !f(key, value, true) {
                    return;
                }
            }
        }

        if remote {
            for (key, value) in self.remotes.iter() {
                if !f(key, value, false) {
                    return;
                }
            }
        }
    }

    pub fn get(&self, hash: &Hash) -> Option<TransactionRef> {
        self.locals
            .get(hash)
            .map(|tx| tx.clone())
            .or(self.remotes.get(hash).map(|tx| tx.clone()))
    }

    pub fn contains(&self, hash: &Hash) -> bool {
        self.locals.contains_key(hash) || self.remotes.contains_key(hash)
    }

    pub fn get_local(&self, hash: &Hash) -> Option<TransactionRef> {
        self.locals
            .get(hash)
            .map(|tx| tx.clone())
    }

    pub fn get_remote(&self, hash: &Hash) -> Option<TransactionRef> {
       self.remotes
            .get(hash)
            .map(|tx| tx.clone())
    }

    pub fn count(&self) -> usize {
        self.locals.len() + self.remotes.len()
    }

    pub fn local_count(&self) -> usize {
        self.locals.len()
    }
    pub fn remote_count(&self) -> usize{
        self.remotes.len()
    }

    pub fn add(&mut self, tx: TransactionRef, local: bool){
        self.slots += num_slots(&tx);
        if local {
            self.locals.insert(tx.hash(), tx);
        } else {
            self.remotes.insert(tx.hash(), tx);
        }
    }
    pub fn remove(&mut self, hash: &Hash) {
        let locals_deleted = self.locals.remove(hash);
        let remotes_deleted = self.remotes.remove(hash);

        if let Some(tx) = locals_deleted {
            self.slots -= num_slots(&tx);
        } else if let Some(tx) = remotes_deleted {
            self.slots -= num_slots(&tx);
        }
    }

    pub fn remote_to_locals(&mut self, local_accounts: &AccountSet) -> Transactions {
        let mut migrated: Vec<TransactionRef> = Vec::new();
        let remotes = self.remotes.clone().into_iter();
        for (hash, tx) in remotes {
            if local_accounts.contains_tx(&tx) {
                self.remotes.remove(&hash);
                self.locals.insert(hash, tx.clone());
                migrated .push(tx.clone())
            }
        }
        migrated
    }

    pub fn slots(&self) -> u64 {
        self.slots
    }
}
