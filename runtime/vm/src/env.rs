use crate::internal::balances_api::BalancesApi;
use crate::internal::blockchain_api::BlockchainApi;
use crate::internal::event::Event;
use crate::internal::storage::Storage;
use crate::Changelist;
use smt::SparseMerkleTree;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{Blockchain, StateDB};
use types::account::{AccountState, Address};

pub struct Env {
    storage: SparseMerkleTree,
    state_db: Arc<dyn StateDB>,
    blockchain: Arc<dyn Blockchain>,
    accounts: HashMap<Address, AccountState>,
    events: Vec<Vec<u8>>,
}

impl Env {
    pub fn new(
        storage: SparseMerkleTree,
        state_db: Arc<dyn StateDB>,
        blockchain: Arc<dyn Blockchain>,
    ) -> Self {
        Self {
            storage,
            state_db,
            blockchain,
            accounts: Default::default(),
            events: vec![],
        }
    }

    fn get_account_state(&mut self, address: Address) -> &mut AccountState {
        let account_state = self
            .accounts
            .entry(address)
            .or_insert_with(|| self.state_db.account_state(&address));
        account_state
    }
}

impl BlockchainApi for Env {
    fn block_hash(&mut self, level: u32) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .blockchain
            .get_block_by_level(level)
            .and_then(|b| b.ok_or(anyhow::anyhow!("block not found")))
            .map(|block| block.hash().to_fixed_bytes().to_vec())
            .unwrap_or_default())
    }
}

impl Storage for Env {
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        let _ = self.storage.update(key, value)?;
        Ok(())
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(Some(self.storage.get(key)?))
    }

    fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<bool> {
        Ok(self.storage.update(key, Vec::new()).is_ok())
    }
}

impl Event for Env {
    fn emit(&mut self, event: Vec<u8>) -> anyhow::Result<()> {
        Ok(self.events.push(event))
    }
}

impl From<Env> for Changelist {
    fn from(value: Env) -> Self {
        Self {
            account_changes: value.accounts,
            logs: value.events,
            storage: codec::Encodable::encode(&value.storage).unwrap_or_default(),
        }
    }
}
