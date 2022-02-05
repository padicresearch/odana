#![feature(async_closure)]
#![feature(trivial_bounds)]

use rocksdb::ColumnFamilyDescriptor;

use storage::Schema;

use crate::block_storage::{BlockByHash, BlockByLevel, BlockPrimaryStorage};
use crate::chain_state::ChainStateStorage;

pub mod block_storage;
pub mod blockchain;
pub mod bootstrap;
pub mod chain_state;
pub mod errors;

pub fn column_families() -> Vec<ColumnFamilyDescriptor> {
    vec![
        BlockPrimaryStorage::descriptor(),
        BlockByLevel::descriptor(),
        BlockByHash::descriptor(),
        ChainStateStorage::descriptor(),
    ]
}

pub fn column_family_names() -> Vec<&'static str> {
    vec![
        BlockPrimaryStorage::column(),
        BlockByLevel::column(),
        BlockByHash::column(),
        ChainStateStorage::column(),
    ]
}
