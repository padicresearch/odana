use std::sync::Arc;

use anyhow::Result;

use storage::{KVStore, Schema};
use types::block::BlockHeader;
use types::ChainStateValue;

pub type ChainStateStorageKV = dyn KVStore<ChainStateStorage> + Send + Sync;

pub struct ChainStateStorage {
    kv: Arc<ChainStateStorageKV>,
}


impl Schema for ChainStateStorage {
    type Key = String;
    type Value = ChainStateValue;

    fn column() -> &'static str {
        "chain_state"
    }
}

const CURR_HEAD: &'static str = "ch";

impl ChainStateStorage {
    pub fn new(kv: Arc<ChainStateStorageKV>) -> Self {
        Self { kv }
    }
    pub fn set_current_header(&self, header: BlockHeader) -> Result<()> {
        self.kv
            .put(CURR_HEAD.to_string(), ChainStateValue::CurrentHeader(header))
    }

    pub fn get_current_header(&self) -> Result<Option<BlockHeader>> {
        let value = self
            .kv
            .get(&CURR_HEAD.to_string())?;

        let r = match value {
            None => {
                None
            }
            Some(ch) => {
                if let ChainStateValue::CurrentHeader(header) = ch {
                    Some(header)
                } else {
                    None
                }
            }
        };
        Ok(r)
    }
}
