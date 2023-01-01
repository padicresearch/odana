#![allow(dead_code)]

use primitive_types::H256;
use std::collections::{BTreeMap, BTreeSet};
use std::iter::FromIterator;

use types::tx::SignedTransaction;
use types::Hash;

use crate::{num_slots, Address, TransactionRef, Transactions};

pub struct AccountSet {
    accounts: BTreeSet<Address>,
}

impl Default for AccountSet {
    fn default() -> Self {
        AccountSet::new()
    }
}

impl From<Vec<Address>> for AccountSet {
    fn from(addresses: Vec<Address>) -> Self {
        let accounts = BTreeSet::from_iter(addresses.into_iter());
        Self { accounts }
    }
}

impl AccountSet {
    pub fn new() -> Self {
        Self {
            accounts: Default::default(),
        }
    }

    pub(crate) fn contains(&self, address: &Address) -> bool {
        self.accounts.contains(address)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub(crate) fn contains_tx(&self, tx: &SignedTransaction) -> bool {
        let address = tx.sender();
        self.contains(&address)
    }

    pub(crate) fn add(&mut self, address: Address) {
        self.accounts.insert(address);
    }

    pub(crate) fn add_tx(&mut self, tx: TransactionRef) {
        let address = tx.sender();
        self.add(address);
    }

    pub(crate) fn flatten(&self) -> Vec<Address> {
        self.accounts.iter().copied().collect()
    }

    pub(crate) fn merge(&mut self, other: &AccountSet) {
        self.accounts.extend(other.accounts.iter());
    }
}

#[derive(Debug, Clone)]
pub struct TxLookup {
    slots: u64,
    locals: BTreeMap<H256, TransactionRef>,
    remotes: BTreeMap<H256, TransactionRef>,
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

impl Default for TxLookup {
    fn default() -> Self {
        TxLookup::new()
    }
}

impl TxLookup {
    pub fn range(&self, f: fn(&H256, &TransactionRef, bool) -> bool, local: bool, remote: bool) {
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

    pub fn get(&self, hash: &H256) -> Option<TransactionRef> {
        self.locals
            .get(hash)
            .cloned()
            .or_else(|| self.remotes.get(hash).cloned())
    }

    pub fn contains(&self, hash: &H256) -> bool {
        self.locals.contains_key(hash) || self.remotes.contains_key(hash)
    }

    pub fn get_local(&self, hash: &H256) -> Option<TransactionRef> {
        self.locals.get(hash).cloned()
    }

    pub fn get_remote(&self, hash: &H256) -> Option<TransactionRef> {
        self.remotes.get(hash).cloned()
    }

    pub fn count(&self) -> usize {
        self.locals.len() + self.remotes.len()
    }

    pub fn local_count(&self) -> usize {
        self.locals.len()
    }
    pub fn remote_count(&self) -> usize {
        self.remotes.len()
    }

    pub fn add(&mut self, tx: TransactionRef, local: bool) {
        let slot = num_slots(&tx);
        self.slots += slot;
        if local {
            self.locals.insert(tx.hash(), tx);
        } else {
            self.remotes.insert(tx.hash(), tx);
        }
    }
    pub fn remove(&mut self, hash: &H256) {
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
        let filtered = self
            .remotes
            .drain_filter(|_hash, tx| local_accounts.contains_tx(tx));
        for (hash, tx) in filtered {
            self.locals.insert(hash, tx.clone());
            migrated.push(tx.clone())
        }
        migrated
    }

    pub fn slots(&self) -> u64 {
        self.slots
    }
}
