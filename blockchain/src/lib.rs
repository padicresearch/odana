#![feature(async_closure)]
#![feature(trivial_bounds)]

use storage::Schema;

use crate::block_storage::{
    BlockByHash, BlockByLevel, BlockHeaderStorage, BlockTransactionsStorage,
};
use crate::chain_state::ChainStateStorage;

pub mod block_storage;
pub mod blockchain;
pub mod chain_state;
pub mod errors;

pub fn column_family_names() -> Vec<&'static str> {
    vec![
        BlockHeaderStorage::column(),
        BlockTransactionsStorage::column(),
        BlockByLevel::column(),
        BlockByHash::column(),
        ChainStateStorage::column(),
    ]
}
