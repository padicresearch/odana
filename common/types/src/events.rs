use crate::block::{Block, BlockHeader};
use crate::tx::Transaction;
use crate::{MempoolSnapsot, Hash};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TxPoolSnapshot {
    pending: Vec<Arc<Hash>>,
    queue: Vec<Arc<Hash>>,
}

#[derive(Clone, Debug)]
pub enum LocalEventMessage {
    MindedBlock(Block),
    BroadcastTx(Transaction),
    TxPoolPack(Vec<Transaction>),
    StateChanged {
        current_head: BlockHeader,
        txpool: TxPoolSnapshot,
    },
}
