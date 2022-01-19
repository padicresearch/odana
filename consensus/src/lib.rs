use types::{Hash, Genesis};
use anyhow::Result;
use types::block::{BlockHeader, Block};
use traits::{StateDB, Consensus, ChainHeadReader};
use std::sync::Arc;
use types::tx::Transaction;
use types::account::AccountState;
use primitive_types::{H160, Compact};
use crate::error::Error;

pub mod coin;
mod error;


pub struct BarossaProtocol {
    genesis: Genesis,
}

impl BarossaProtocol {
    const MAX_BLOCK_HEIGHT: u128 = 25_000_000;
    const INITIAL_REWARD: u128 = 10 * 1_000_000_000 /*TODO: Use TUC constant*/;
    const SPREAD: u128 = MAX_BLOCK_HEIGHT.pow(4) / INITIAL_REWARD;
    const PRECISION_CORRECTION: u128 = 5012475762;
    const MAX_SUPPLY_APPROX: u128 = (INITIAL_REWARD * MAX_BLOCK_HEIGHT) - (MAX_BLOCK_HEIGHT.pow(5) / (5 * SPREAD));
    const MAX_SUPPLY_PRECOMPUTED: u128 = MAX_SUPPLY_APPROX + PRECISION_CORRECTION;

    const NONCE: u128 = 0;
    const DIFFICULTY: u32 = 3;

    #[inline]
    fn miner_reward(block_height: u128) -> u128 {
        Self::INITIAL_REWARD - block_height.pow(4) / Self::SPREAD
    }

    #[inline]
    fn total_supply_at_block(block_height: u128) -> u128 {
        (Self::INITIAL_REWARD * block_height) - (block_height.pow(5) / (5 * Self::SPREAD))
    }

    fn verify_head(&self) -> Result<()> {}
}

impl Consensus for BarossaProtocol {
    fn verify_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<()> {
        let parent_header = chain.get_header(header.parent_hash(), header.level() - 1)?.ok_or(Error::ParentBlockNotFound)?;
        if header.d() != Self::NONCE {}

        todo!()
    }

    fn prepare_header(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader) -> Result<BlockHeader> {
        todo!()
    }

    fn finalize(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<()> {
        todo!()
    }

    fn finalize_and_assemble(&self, chain: Arc<dyn ChainHeadReader>, header: &BlockHeader, state: Arc<dyn StateDB>, txs: Vec<Transaction>) -> Result<Option<Block>> {
        todo!()
    }

    fn calc_difficulty(&self, level: i32, parent: &BlockHeader) -> Compact {
        todo!()
    }

    fn is_genesis(&self, header: &BlockHeader) -> bool {
        todo!()
    }
}