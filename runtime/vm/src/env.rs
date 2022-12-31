use crate::runtime::runtime::Runtime;
use crate::state::state::State;
use crate::storage::storage::Storage;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{Blockchain, StateDB};
use types::account::{AccountState, Address42};

pub struct Env {
    state_db: Arc<dyn StateDB>,
    blockchain: Arc<dyn Blockchain>,
    accounts: HashMap<Address42, AccountState>,
}

impl Env {
    pub fn new(state_db: Arc<dyn StateDB>, blockchain: Arc<dyn Blockchain>) -> Self {
        Self {
            state_db,
            blockchain,
            accounts: Default::default(),
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

impl State for Env {
    fn get_nonce(
        &mut self,
        address: Vec<u8>,
    ) -> wasmtime::component::__internal::anyhow::Result<u64> {
        Address42::from_slice(&address).map(|address| self.get_account_state(address).nonce)
    }

    fn get_free_balance(
        &mut self,
        address: Vec<u8>,
    ) -> wasmtime::component::__internal::anyhow::Result<u64> {
        Address42::from_slice(&address).map(|address| self.get_account_state(address).free_balance)
    }

    fn get_reserve_balance(
        &mut self,
        address: Vec<u8>,
    ) -> wasmtime::component::__internal::anyhow::Result<u64> {
        Address42::from_slice(&address)
            .map(|address| self.get_account_state(address).reserve_balance)
    }

    fn add_free_balance(
        &mut self,
        address: Vec<u8>,
        amount: u64,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.free_balance += amount;
        Ok(())
    }

    fn sub_free_balance(
        &mut self,
        address: Vec<u8>,
        amount: u64,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.free_balance -= amount;
        Ok(())
    }

    fn add_reserve_balance(
        &mut self,
        address: Vec<u8>,
        amount: u64,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.reserve_balance += amount;
        Ok(())
    }

    fn sub_reserve_balance(
        &mut self,
        address: Vec<u8>,
        amount: u64,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        let account_state =
            Address42::from_slice(&address).map(|address| self.get_account_state(address))?;
        account_state.reserve_balance -= amount;
        Ok(())
    }
}

impl Runtime for Env {
    fn on_event(
        &mut self,
        event: String,
        params: Vec<String>,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        todo!()
    }

    fn finality_block_level(&mut self) -> wasmtime::component::__internal::anyhow::Result<u32> {
        todo!()
    }

    fn block_hash(
        &mut self,
        level: u32,
    ) -> wasmtime::component::__internal::anyhow::Result<Vec<u8>> {
        todo!()
    }
}

impl Storage for Env {
    fn insert(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> wasmtime::component::__internal::anyhow::Result<()> {
        todo!()
    }

    fn get(
        &mut self,
        key: Vec<u8>,
    ) -> wasmtime::component::__internal::anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    fn remove(&mut self, key: Vec<u8>) -> wasmtime::component::__internal::anyhow::Result<bool> {
        todo!()
    }

    fn root(&mut self) -> wasmtime::component::__internal::anyhow::Result<Vec<u8>> {
        todo!()
    }
}
