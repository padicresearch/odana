use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize};

use anyhow::Result;

use primitive_types::H160;
use tracing::stdlib::iter::FromIterator;
use types::Hash;
use types::tx::Transaction;

use crate::{Address, num_slots, TransactionRef, Transactions};

pub struct AccountSet {
    accounts: RwLock<BTreeSet<H160>>,
}

impl From<Vec<Address>> for AccountSet {
    fn from(addresses: Vec<Address>) -> Self {
        let accounts = BTreeSet::from_iter(addresses.into_iter());
        Self {
            accounts: RwLock::new(accounts)
        }
    }
}

impl AccountSet {
    pub(crate) fn new() -> Self {
        Self {
            accounts: Default::default()
        }
    }

    pub(crate) fn contains(&self, address: &H160) -> Result<bool> {
        let accounts = self.accounts.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(accounts.contains(address))
    }

    pub(crate) fn is_empty(&self, address: &H160) -> Result<bool> {
        let accounts = self.accounts.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(accounts.is_empty())
    }

    pub(crate) fn contains_tx(&self, tx: &Transaction) -> Result<bool> {
        let address = tx.sender();
        self.contains(&address)
    }

    pub(crate) fn add(&self, address: H160) -> Result<()> {
        let mut accounts = self.accounts.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        accounts.insert(address);
        Ok(())
    }

    pub(crate) fn add_tx(&self, tx: TransactionRef) -> Result<()> {
        let address = tx.sender();
        self.add(address);
        Ok(())
    }

    pub(crate) fn flatten(&self) -> Result<Vec<H160>> {
        let mut accounts = self.accounts.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(accounts.iter().map(|addrs| addrs.clone()).collect())
    }
}

pub struct TxLookup {
    slots: AtomicU64,
    locals: Arc<RwLock<BTreeMap<Hash, TransactionRef>>>,
    remotes: Arc<RwLock<BTreeMap<Hash, TransactionRef>>>,
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
    ) -> Result<()> {
        if local {
            let mut txs = self.locals.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            for (key, value) in txs.iter() {
                if !f(key, value, true) {
                    return Ok(());
                }
            }
        }

        if remote {
            let mut txs = self.remotes.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            for (key, value) in txs.iter() {
                if !f(key, value, false) {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, hash: &Hash) -> Result<Option<TransactionRef>> {
        let mut locals = self.locals.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut remotes = self.remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(locals
            .get(hash)
            .map(|tx| tx.clone())
            .or(remotes.get(hash).map(|tx| tx.clone())))
    }

    pub fn contains(&self, hash: &Hash) -> Result<bool> {
        let mut locals = self.locals.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut remotes = self.remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(locals.contains_key(hash) || remotes.contains_key(hash))
    }

    pub fn get_local(&self, hash: &Hash) -> Result<Option<TransactionRef>> {
        let mut locals = self.locals.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(locals
            .get(hash)
            .map(|tx| tx.clone()))
    }

    pub fn get_remote(&self, hash: &Hash) -> Result<Option<TransactionRef>> {
        let mut remotes = self.remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(remotes
            .get(hash)
            .map(|tx| tx.clone()))
    }

    pub fn count(&self, hash: &Hash) -> Result<usize> {
        let mut locals = self.locals.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut remotes = self.remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(locals.len() + remotes.len())
    }

    pub fn local_count(&self, hash: &Hash) -> Result<usize> {
        let mut locals = self.locals.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(locals.len())
    }
    pub fn remote_count(&self, hash: &Hash) -> Result<usize> {
        let mut remotes = self.remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(remotes.len())
    }

    pub fn add(&self, tx: TransactionRef, local: bool) -> Result<()> {
        self.slots.fetch_add(num_slots(&tx), std::sync::atomic::Ordering::Relaxed);
        if local {
            let mut locals = self.locals.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            locals.insert(tx.hash(), tx);
        } else {
            let mut remotes = self.remotes.write().map_err(|e| anyhow::anyhow!("{}", e))?;
            remotes.insert(tx.hash(), tx);
        }
        Ok(())
    }
    pub fn remove(&self, hash: &Hash) -> Result<()> {
        let mut locals = self.locals.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut remotes = self.remotes.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let locals_deleted = locals.remove(hash);
        let remotes_deleted = locals.remove(hash);

        if let Some(tx) = locals_deleted {
            self.slots.fetch_sub(num_slots(&tx), std::sync::atomic::Ordering::Relaxed);
        } else if let Some(tx) = remotes_deleted {
            self.slots.fetch_sub(num_slots(&tx), std::sync::atomic::Ordering::Relaxed);
        }
        return Ok(());
    }

    pub fn remote_to_locals(&self, local_accounts: &AccountSet) -> Result<Transactions> {
        let remotes = self.remotes.clone();
        let mut remotes = remotes.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut migrated: Vec<TransactionRef> = Vec::new();

        for (hash, tx) in remotes.iter() {
            if local_accounts.contains_tx(tx)? {
                let mut locals = self.locals.write().map_err(|e| anyhow::anyhow!("{}", e))?;
                let mut remotes = self.remotes.write().map_err(|e| anyhow::anyhow!("{}", e))?;
                locals.insert(*hash, tx.clone());
                remotes.remove(hash);
                migrated .push(tx.clone())
            }
        }
        Ok(migrated)
    }

    pub fn slots(&self) -> u64 {
        self.slots.load(std::sync::atomic::Ordering::Relaxed)
    }
}
