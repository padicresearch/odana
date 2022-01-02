use crate::blockchain::BlockChainState;
use crate::errors::BlockChainError;
use crate::mempool::MemPool;
use crate::miner::Miner;
use crate::transaction::Tx;
use crate::utxo::UTXO;
use anyhow::{Error, Result};
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use types::block::Block;

pub struct ChainManager {
    mempool: Arc<MemPool>,
}

impl ChainManager {
    pub fn broadcast_tx_to_peers(&self, tx: Tx) -> Result<()> {
        if self.mempool.contains(tx.id())? {
            return Ok(());
        }
        // Broadcast to peers
        todo!()
    }

    pub fn on_recv_tx_from_peers(&self, tx: Tx) -> Result<()> {
        todo!()
    }

    pub fn broadcast_block_to_peers(&self, tx: Block) -> Result<()> {
        todo!()
    }

    pub fn on_recv_block_from_peers(&self, tx: Block) -> Result<()> {
        todo!()
    }
}
