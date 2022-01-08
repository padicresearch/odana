use crate::block::{Block, BlockHeader};
use crate::tx::Transaction;
use crate::{MempoolSnapsot, TxHash};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TxPoolSnapshot {
    pending: Vec<Arc<TxHash>>,
    queue: Vec<Arc<TxHash>>,
}

#[derive(Clone, Debug)]
pub enum LocalEventMessage {
    MindedBlock(Block),
    BroadcastTx(Transaction),
    TxPoolPack(Vec<Transaction>),
    TxPoolRunPack,
    StateChanged {
        current_head: BlockHeader,
        txpool: TxPoolSnapshot,
    },
}
