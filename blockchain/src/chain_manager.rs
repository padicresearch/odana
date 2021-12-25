use anyhow::{Result, Error};
use crate::transaction::Tx;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::utxo::UTXO;
use std::sync::Arc;
use crate::mempool::MemPool;
use crate::account::Account;
use crate::block::Block;
use crate::miner::Miner;
use crate::blockchain::BlockChainState;
use crate::errors::BlockChainError;

pub struct ChainManager {
    mempool : Arc<MemPool>
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

pub fn start_mining(miner: Arc<Miner>, state: Arc<BlockChainState>, sender: UnboundedSender<Block>)  {
    tokio::task::spawn( async move {
        loop {
            let state = state.clone();
            match miner.mine(&state.get_current_head().expect("Blockchain state failed").ok_or(BlockChainError::UnknownError).expect("Blockchain state failed")) {
                Ok(new_block) => {
                    sender.send(new_block);
                }
                Err(error) => {
                    println!("Miner Error: {}", error);
                }
            }
        }
    });
}
