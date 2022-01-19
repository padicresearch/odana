use types::block::{BlockHeader, Block};
use anyhow::Result;
use types::account::AccountState;
use std::sync::Arc;
use primitive_types::H160;
use traits::StateDB;

pub trait Blockchain: Send + Sync {
    fn current_head(&self) -> Result<BlockHeader>;
    fn get_block(&self, block_hash: &types::Hash) -> Result<Option<Block>>;
    fn get_state_at(&self, root: &types::Hash) -> Result<Arc<dyn StateDB>>;
    fn get_current_state(&self) -> Result<Arc<dyn StateDB>>;
}