use crate::internal::event::Event;
use crate::internal::execution_context::ExecutionContext;
use crate::internal::log::Log;
use crate::internal::storage::Storage;
use crate::internal::syscall::Syscall;
use anyhow::anyhow;
use crypto::ecdsa::{PublicKey, SecretKey};
use primitive_types::Address;
use smt::SparseMerkleTree;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{Blockchain, ChainHeadReader, StateDB};
use types::account::{get_address_from_pub_key, get_address_from_seed, AccountState};
use types::network::Network;
use types::{Addressing, Changelist};

pub struct ExecutionEnvironment {
    network: Network,
    sender: Address,
    app_address: Address,
    value: u64,
    storage: SparseMerkleTree,
    state_db: Arc<dyn StateDB>,
    blockchain: Arc<dyn ChainHeadReader>,
    accounts: HashMap<Address, AccountState>,
    events: Vec<Vec<u8>>,
}

impl ExecutionEnvironment {
    pub fn new(
        origin: Address,
        app_id: Address,
        value: u64,
        storage: SparseMerkleTree,
        state_db: Arc<dyn StateDB>,
        blockchain: Arc<dyn ChainHeadReader>,
    ) -> anyhow::Result<ExecutionEnvironment> {
        let mut accounts = HashMap::new();
        let mut account_state = state_db.account_state(&app_id);
        account_state.free_balance += value;
        accounts.insert(app_id, account_state);

        Ok(Self {
            network: app_id.network().ok_or(anyhow!("network not found"))?,
            sender: origin,
            app_address: app_id,
            value,
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

impl Syscall for ExecutionEnvironment {
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
        PublicKey::from_bytes(&pk)
            .map(|pk| get_address_from_pub_key(pk, self.network))
            .map(|add| add.as_bytes().to_vec())
            .map_err(|e| anyhow!(e))
    }

    fn generate_keypair(&mut self) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let account = account::create_account(self.network);
        Ok((
            account.secrete_key().as_bytes().to_vec(),
            account.public_key().as_bytes().to_vec(),
        ))
    }

    fn generate_native_address(&mut self, seed: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        get_address_from_seed(&seed, self.network).map(|address| address.to_vec())
    }

    fn sign(&mut self, sk: Vec<u8>, msg: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let sk = SecretKey::from_bytes(&sk)?;
        sk.sign(&msg)
            .map(|sig| sig.to_bytes().to_vec())
            .map_err(|e| e.into())
    }

    fn transfer(&mut self, to: Vec<u8>, amount: u64) -> anyhow::Result<bool> {
        //TODO: very unsafe
        let to = Address::from_slice(&to).map_err(|_| anyhow!("invalid address"))?;
        let from_acc = self.get_account_state(self.app_address);
        anyhow::ensure!((from_acc.free_balance as i128 - amount as i128) > 0);
        from_acc.free_balance -= amount;
        let to_acc = self.get_account_state(to);
        to_acc.free_balance += amount;
        Ok(true)
    }

    fn reserve(&mut self, amount: u64) -> anyhow::Result<bool> {
        //TODO: very unsafe
        let from_acc = self.get_account_state(self.sender);
        anyhow::ensure!((from_acc.free_balance as i128 - amount as i128) > 0);
        from_acc.free_balance -= amount;
        from_acc.reserve_balance += amount;
        Ok(true)
    }

    fn unreserve(&mut self, amount: u64) -> anyhow::Result<bool> {
        //TODO: very unsafe
        let from_acc = self.get_account_state(self.sender);
        anyhow::ensure!((from_acc.reserve_balance as i128 - amount as i128) > 0);
        from_acc.reserve_balance -= amount;
        from_acc.free_balance += amount;
        Ok(true)
    }
}

impl Storage for ExecutionEnvironment {
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

impl Event for ExecutionEnvironment {
    fn emit(&mut self, event: Vec<u8>) -> anyhow::Result<()> {
        Ok(self.events.push(event))
    }
}

impl Log for ExecutionEnvironment {
    fn print(&mut self, output: Vec<char>) -> anyhow::Result<()> {
        println!("{:?}", output);
        Ok(())
    }
}

impl ExecutionContext for ExecutionEnvironment {
    fn value(&mut self) -> anyhow::Result<u64> {
        Ok(self.value)
    }

    fn block_level(&mut self) -> anyhow::Result<u32> {
        todo!()
    }

    fn sender(&mut self) -> anyhow::Result<Vec<u8>> {
        Ok(self.sender.to_vec())
    }

    fn network(&mut self) -> anyhow::Result<u32> {
        Ok(self.network.chain_id())
    }

    fn sender_pk(&mut self) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

impl From<ExecutionEnvironment> for Changelist {
    fn from(value: ExecutionEnvironment) -> Self {
        Self {
            account_changes: value.accounts,
            logs: value.events,
            storage: value.storage,
        }
    }
}
