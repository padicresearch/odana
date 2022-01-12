use anyhow::Result;
use primitive_types::H160;
use std::sync::Arc;
use types::account::AccountState;
use types::block::{Block, BlockHeader};
use types::{BlockHash, PubKey, TxHash};
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

pub trait ChainState: Send + Sync {
    fn current_head(&self) -> Result<BlockHeader>;
    fn get_block(&self, block_hash: &types::Hash) -> Result<Option<Block>>;
    fn get_state_at(&self, root: &types::Hash) -> Result<Arc<dyn StateDB>>;
}

pub trait StateDB: Send + Sync {
    fn account_nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
}

pub trait Saturating {
    fn saturating_add(self, rhs: Self) -> Self;

    fn saturating_sub(self, rhs: Self) -> Self;

    fn saturating_mul(self, rhs: Self) -> Self;
}
