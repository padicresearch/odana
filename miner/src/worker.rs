use std::sync::{Arc, RwLock};
use traits::Consensus;
use txpool::TxPool;

pub fn start_mine_worker(consensus: Arc<dyn Consensus>, tx_pool: Arc<RwLock<TxPool<>>>)