use types::block::{Block, BlockHeader};
use types::{PubKey, BlockHash, TxHash};
use types::account::AccountState;
use anyhow::Result;
use primitive_types::H160;
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

pub trait BlockchainState: Send + Sync + Clone {
    fn current_head(&self) -> Result<Option<BlockHeader>>;
}

pub trait StateDB: Send + Sync + Clone {
    fn account_nonce(&self, address: &H160) -> u64;
    fn account_state(&self, address: &H160) -> AccountState;
}