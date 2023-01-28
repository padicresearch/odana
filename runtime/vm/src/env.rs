use crate::internal::context::Context;
use crate::internal::event::Event;
use crate::internal::log::Log;
use crate::internal::storage::Storage;
use crate::internal::syscall::Syscall;
use primitive_types::Address;
use smt::SparseMerkleTree;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{Blockchain, ChainHeadReader, StateDB};
use types::account::AccountState;
use types::Changelist;

pub struct Env<'a> {
    storage: SparseMerkleTree,
    state_db: &'a dyn StateDB,
    blockchain: Arc<dyn ChainHeadReader>,
    accounts: HashMap<Address, AccountState>,
    events: Vec<Vec<u8>>,
}

impl<'a> Env<'a> {
    pub fn new(
        app_id: Address,
        value: u64,
        storage: SparseMerkleTree,
        state_db: &'a dyn StateDB,
        blockchain: Arc<dyn ChainHeadReader>,
    ) -> anyhow::Result<Env> {
        let mut accounts = HashMap::new();
        let mut account_state = state_db.account_state(&app_id);
        account_state.free_balance += value;
        accounts.insert(app_id, account_state);

        Ok(Self {
            storage,
            state_db,
            blockchain,
            accounts,
            events: vec![],
        })
    }

    fn get_account_state(&mut self, address: Address) -> &mut AccountState {
        let account_state = self
            .accounts
            .entry(address)
            .or_insert_with(|| self.state_db.account_state(&address));
        account_state
    }
}

impl<'a> Syscall for Env<'a> {
    fn block_hash(&mut self, level: u32) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .blockchain
            .get_header_by_level(level)
            .and_then(|b| b.ok_or(anyhow::anyhow!("block not found")))
            .map(|block| block.hash.as_bytes().to_vec())
            .unwrap_or_default())
    }

    fn block(&mut self, block_hash: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn address_from_pk(&mut self, pk: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn generate_keypair(&mut self) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        todo!()
    }

    fn generate_native_address(&mut self, seed: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn sign(&mut self, sk: Vec<u8>, msg: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn transfer(&mut self, to: Vec<u8>, amount: u64) -> anyhow::Result<bool> {
        todo!()
    }

    fn reserve(&mut self, amount: u64) -> anyhow::Result<bool> {
        todo!()
    }
}

impl<'a> Storage for Env<'a> {
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

impl<'a> Event for Env<'a> {
    fn emit(&mut self, event: Vec<u8>) -> anyhow::Result<()> {
        Ok(self.events.push(event))
    }
}

impl<'a> Log for Env<'a> {
    fn print(&mut self, output: Vec<char>) -> anyhow::Result<()> {
        println!("{:?}", output);
        Ok(())
    }
}

impl<'a> Context for Env<'a> {
    fn call_value(&mut self) -> anyhow::Result<u64> {
        todo!()
    }

    fn caller_address(&mut self) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn caller_pk(&mut self) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

impl<'a> From<Env<'a>> for Changelist {
    fn from(value: Env) -> Self {
        Self {
            account_changes: value.accounts,
            logs: value.events,
            storage: value.storage,
        }
    }
}
