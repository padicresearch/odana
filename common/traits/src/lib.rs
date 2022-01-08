use anyhow::Result;
use primitive_types::H160;
use types::account::AccountState;
use types::block::{Block, BlockHeader};
use types::{BlockHash, PubKey, TxHash};
use std::sync::Arc;
// pub trait SudoAccount {
//     fn is_sudo(&self, account: &AccountId) -> bool;
//     fn sudo(&self) -> AccountId;
// }
//
// pub trait TreasuryAccount {
//     fn treasury(&self) -> AccountId;
// }
//
// pub trait Txs {
//     fn get_transaction(&self, hash : &TxHash) -> Transaction;
// }
//
//
// pub trait Blocks {
//     fn get_block(&self, hash : &BlockHash) -> Block;
// }

pub trait ChainState: Send + Sync + Clone {
    fn current_head(&self) -> Result<BlockHeader>;
    fn get_block(&self, block_hash: &types::Hash) -> Result<Option<Block>>;
    fn get_state_at(&self, root: &types::Hash) -> Result<dyn StateDB>;
}

pub trait StateDB: Send + Sync + Clone {
    fn account_nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
}
