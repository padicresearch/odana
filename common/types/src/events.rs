use crate::block::{Block, BlockHeader};
use crate::tx::SignedTransaction;

#[derive(Clone, Debug)]
pub enum LocalEventMessage {
    MindedBlock(Block),
    BroadcastTx(Vec<SignedTransaction>),
    TxPoolPack(Vec<SignedTransaction>),
    StateChanged {
        current_head: BlockHeader,
    },
    NetworkHighestHeadChanged {
        peer_id: String,
        tip: Option<BlockHeader>,
    },
    NetworkNewPeerConnection {
        stats: (usize, usize),
        peer_id: String,
    },
}
