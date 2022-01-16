use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::AtomicUsize;

use anyhow::Result;

use types::tx::Transaction;
use types::TxHash;

use crate::{TransactionRef, TxHashRef};

pub struct TxList {
    txs: BTreeMap<Reverse<u64>, TransactionRef>,
}

pub type TransactionIterator<'a> = Box<dyn 'a + Send + Iterator<Item = TransactionRef>>;

impl TxList {
    pub fn new() -> Self {
        Self {
            txs: Default::default(),
        }
    }
    pub fn put(&mut self, tx: TransactionRef) -> Option<TransactionRef>{self.txs.insert(std::cmp::Reverse(tx.nonce()), tx)
    }

    pub fn remove(&mut self, tx: TransactionRef) ->Option<TransactionRef> {
        self.txs.remove(&Reverse(tx.nonce()))
    }

    pub fn forward(&mut self, threshold: u64) -> Vec<TransactionRef> {
        let mut removed = self.txs.split_off(&Reverse(threshold - 1));
        removed.iter().map(|tx| tx.1.clone()).collect()
    }

    pub fn ready(&mut self, start: u64) -> Vec<TransactionRef> {
        let mut ready = self.txs.split_off(&Reverse(start));
        std::mem::swap(&mut ready, &mut self.txs);
        self.txs.clear();
        ready.iter().map(|tx| tx.1.clone()).collect()
    }

    pub fn cap(&mut self, threshold: usize) -> Vec<TransactionRef>{
        if self.txs.len() <= threshold {
            return Default::default();
        }
        let mut remain = BTreeMap::new();
        let mut slots = threshold;
        while slots > 0 {
            if let Some((tx_hash, tx)) = self.txs.pop_first() {
                remain.insert(tx_hash, tx);
                slots -= 1;
            } else {
                break;
            }
        }
        std::mem::swap(&mut remain, &mut self.txs);
        let drops: Vec<_> = remain
            .iter()
            .map(|(_, priced_tx)| priced_tx.clone())
            .collect();
        drops
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn flatten(&self) -> Vec<TransactionRef> {
        self.txs.iter().map(|(_,tx)| tx.clone()).collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PricedTransaction(TransactionRef);

impl Eq for PricedTransaction {}

impl PartialEq for PricedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for PricedTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.0.fees().partial_cmp(&other.0.fees()) {
            None => None,
            Some(comp) => match comp.reverse() {
                Ordering::Less => Some(Ordering::Less),
                Ordering::Equal => self
                    .0
                    .nonce()
                    .partial_cmp(&other.0.nonce())
                    .map(|ord| ord.reverse()),
                Ordering::Greater => Some(Ordering::Greater),
            },
        }
    }
}

impl Ord for PricedTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.fees().cmp(&other.0.fees()).reverse() {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.0.nonce().cmp(&other.0.nonce()).reverse(),
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[derive(Debug)]
pub(crate) struct NonceTransaction(TransactionRef);

impl Eq for NonceTransaction {}

impl PartialEq for NonceTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for NonceTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.nonce().cmp(&other.0.nonce()).reverse())
    }
}

impl Ord for NonceTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.nonce().cmp(&other.0.nonce()).reverse()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TxPricedList {
    txs: Arc<RwLock<BTreeSet<PricedTransaction>>>,
    total_fee: Arc<AtomicUsize>,
}

impl TxPricedList {
    pub fn new() -> Self {
        Self {
            txs: Default::default(),
            total_fee: Default::default(),
        }
    }

    pub fn put(&self, tx: TransactionRef, is_local: bool) -> Result<()> {
        if is_local {
            return Ok(());
        }
        let mut txs = self.txs.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.total_fee
            .fetch_add(tx.fees() as usize, std::sync::atomic::Ordering::Relaxed);
        let _ = txs.insert(PricedTransaction(tx));
        Ok(())
    }

    pub fn remove(&self, tx: TransactionRef) -> Result<bool> {
        let mut txs = self.txs.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        let tx_fee = tx.fees() as usize;
        let removed = txs.remove(&PricedTransaction(tx));
        if removed {
            self.total_fee
                .fetch_sub(tx_fee, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(removed)
    }

    pub fn underpriced(&self, tx: TransactionRef) -> Result<bool> {
        let mut txs = self.txs.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let least_priced_tx = match txs.last() {
            None => {
                return Ok(false);
            }
            Some(tx) => tx,
        };
        Ok(least_priced_tx.cmp(&PricedTransaction(tx)).is_ge())
    }

    pub fn discard(&self, slots: usize) -> Result<Vec<TransactionRef>> {
        let mut txs = self.txs.write().map_err(|e| anyhow::anyhow!("{}", e))?;
        if txs.len() <= slots {
            return Ok(Default::default());
        }
        let mut remain = BTreeSet::new();
        let mut slots = slots;
        let mut iter = txs.iter();
        while slots > 0 {
            if let Some(tx) = txs.pop_first() {
                remain.insert(tx);
                slots -= 1;
            } else {
                break;
            }
        }
        std::mem::swap(&mut remain, &mut txs);
        let drops: Vec<_> = remain.iter().map(|priced_tx| priced_tx.0.clone()).collect();
        Ok(drops)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BinaryHeap, BTreeSet};
    use std::sync::Arc;

    use account::create_account;
    use transaction::make_sign_transaction;
    use types::account::Account;
    use types::tx::{Transaction, TransactionKind};

    use crate::TransactionRef;
    use crate::tx_list::{PricedTransaction, TxList, TxPricedList};

    fn make_tx(
        from: &Account,
        to: &Account,
        nonce: u64,
        amount: u128,
        fee: u128,
    ) -> TransactionRef {
        let tx = make_sign_transaction(
            from,
            nonce,
            TransactionKind::Transfer {
                from: from.pub_key,
                to: to.pub_key,
                amount,
                fee,
            },
        )
        .unwrap();
        Arc::new(tx)
    }

    #[test]
    fn test_txlist() {
        let alice = create_account();
        let bob = create_account();
        let mut list = TxList::new();
        list.put(make_tx(&alice, &bob, 1, 100, 0));
        list.put(make_tx(&alice, &bob, 2, 100, 0));
        list.put(make_tx(&alice, &bob, 3, 100, 0));
        list.put(make_tx(&alice, &bob, 5, 100, 0));
        list.put(make_tx(&alice, &bob, 6, 100, 0));
        list.put(make_tx(&alice, &bob, 7, 100, 0));
        list.put(make_tx(&alice, &bob, 8, 100, 0));
        list.put(make_tx(&alice, &bob, 9, 100, 0));

        let removed = list.forward(3);
        let readies = list.ready(3);
        println!("forward {:#?}", removed);
        println!("ready {:#?}", readies);
        println!("remaining {:#?}", list.txs);
        // assert_eq!(removed.len(), 2);
        // assert_eq!(removed.range(3..).count(), 0);
        // assert_eq!(list.txs.write().unwrap().range(..3).count(), 0);

        let mut priced_list = TxPricedList::new();
        priced_list
            .put(make_tx(&alice, &bob, 1, 40, 4), false)
            .unwrap();
        priced_list
            .put(make_tx(&alice, &bob, 2, 20, 2), false)
            .unwrap();
        priced_list
            .put(make_tx(&alice, &bob, 3, 30, 3), false)
            .unwrap();
        priced_list
            .put(make_tx(&alice, &bob, 4, 40, 4), false)
            .unwrap();
        priced_list
            .put(make_tx(&bob, &alice, 8, 100, 10), false)
            .unwrap();
        priced_list
            .put(make_tx(&bob, &alice, 9, 100, 10), false)
            .unwrap();

        // println!("{:#?}", priced_list);
        // println!("-------------------------------------------------------------------------------------------------------");
        // println!("{:#?}", priced_list.discard(4).unwrap());
    }
}
