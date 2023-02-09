#![feature(async_closure)]
#![feature(trivial_bounds)]

use rocksdb::ColumnFamilyDescriptor;

use storage::{default_table_options, Schema};

use crate::block_storage::{
    BlockByHash, BlockByLevel, BlockHeaderStorage, BlockTransactionsStorage,
};
use crate::chain_state::ChainStateStorage;

pub mod block_storage;
pub mod blockchain;
pub mod chain_state;
pub mod errors;

pub fn column_families() -> Vec<ColumnFamilyDescriptor> {
    column_family_names()
        .into_iter()
        .map(|name| ColumnFamilyDescriptor::new(name, default_table_options()))
        .collect()
}

pub fn column_family_names() -> Vec<&'static str> {
    vec![
        BlockHeaderStorage::column(),
        BlockTransactionsStorage::column(),
        BlockByLevel::column(),
        BlockByHash::column(),
        ChainStateStorage::column(),
    ]
}
