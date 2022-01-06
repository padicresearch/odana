// use std::collections::BTreeMap;
// use crate::TransactionRef;
// use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
// use anyhow::Result;
//
// type SortedMap = BTreeMap<u64, TransactionRef>;
//
// pub struct TxList {
//     txs : Arc<RwLock<SortedMap>>
// }
//
// impl TxList {
//     fn txs_reader(&self) -> Result<RwLockReadGuard<SortedMap>> {
//         let txs = self.txs.clone();
//         Ok(txs.read()?)
//     }
//
//     fn txs_writer(&self) -> Result<RwLockWriteGuard<SortedMap>> {
//         let txs = self.txs.clone();
//         Ok(txs.write()?)
//     }
// }
//
// impl TxList {
//
//     pub fn add(&self, tx : TransactionRef) -> Result<()> {
//         self.txs_writer()?.en
//     }
// }