use std::sync::Arc;

use crate::{Hash, MempoolSnapsot};
use crate::block::{Block, BlockHeader};
use crate::tx::Transaction;

#[derive(Clone, Debug)]
pub enum LocalEventMessage {
    MindedBlock(Block),
    BroadcastTx(Transaction),
    TxPoolPack(Vec<Transaction>),
    StateChanged { current_head: BlockHeader },
}
