use crate::internal::balances_api::BalancesApi;
use crate::internal::blockchain_api::BlockchainApi;
use crate::internal::event::Event;
use crate::internal::storage::Storage;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{Blockchain, StateDB};
use types::account::{AccountState, Address42};

pub struct Env {
    state_db: Arc<dyn StateDB>,
    blockchain: Arc<dyn Blockchain>,
    accounts: HashMap<Address42, AccountState>,
    events: Vec<Vec<u8>>,
}

impl Env {
    pub fn new(state_db: Arc<dyn StateDB>, blockchain: Arc<dyn Blockchain>) -> Self {
        Self {
            state_db,
            blockchain,
            accounts: Default::default(),
            events: vec![],
        }
    }

    fn get_account_state(&mut self, address: Address42) -> &mut AccountState {
        let account_state = self
            .accounts
            .entry(address)
            .or_insert_with(|| self.state_db.account_state(&address));
        account_state
    }

    pub(crate) fn account_changes(&self) -> &HashMap<Address42, AccountState> {
        &self.accounts
    }
}

impl BalancesApi for Env {
    fn get_free_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Address42::from_slice(&address).map(|address| self.get_account_state(address).free_balance)
    }

    fn get_reserve_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Address42::from_slice(&address)
            .map(|address| self.get_account_state(address).reserve_balance)
    }

    fn add_free_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.free_balance += amount;
        Ok(())
    }

    fn sub_free_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.free_balance -= amount;
        Ok(())
    }

    fn add_reserve_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.reserve_balance += amount;
        Ok(())
    }

    fn sub_reserve_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.reserve_balance -= amount;
        Ok(())
    }
}

impl BlockchainApi for Env {
    fn finality_block_level(&mut self) -> anyhow::Result<u32> {
        self.blockchain
            .current_header()
            .and_then(|b| b.ok_or(anyhow::anyhow!("current head not available")))
            .map(|block| block.raw.level().saturating_sub(60))
    }

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
        todo!()
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<bool> {
        todo!()
    }
}

impl Event for Env {
    fn emit(&mut self, event: Vec<u8>) -> anyhow::Result<()> {
        Ok(self.events.push(event))
    }
}
