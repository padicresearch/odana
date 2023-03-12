use crate::internal::event::Event;
use crate::internal::execution_context::ExecutionContext;
use crate::internal::logging::Logging;
use crate::internal::storage::Storage;
use crate::internal::syscall::Syscall;
use anyhow::{anyhow, bail};
use crypto::ecdsa::{PublicKey, SecretKey};
use primitive_types::address::Address;
use smt::SparseMerkleTree;
use std::collections::HashMap;
use std::sync::Arc;
use traits::{ChainHeadReader, StateDB};
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
    events: Vec<(String, Vec<u8>)>,
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
            network: app_id
                .network()
                .ok_or_else(|| anyhow!("network not found"))?,
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
            .and_then(|b| b.ok_or_else(|| anyhow::anyhow!("block not found")))
            .map(|block| block.hash.as_bytes().to_vec())
            .unwrap_or_default())
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
        let to = Address::from_slice(&to);
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

    fn get_free_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .free_balance)
    }

    fn get_nonce(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .nonce)
    }

    fn get_reserve_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .reserve_balance)
    }
}

impl Storage for ExecutionEnvironment {
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        let _ = self.storage.update(key, value)?;
        Ok(())
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let Ok(data) = self.storage.get(key) else {
            return Ok(None)
        };
        Ok(Some(data))
    }

    fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<bool> {
        Ok(self.storage.update(key, Vec::new()).is_ok())
    }
}

impl Event for ExecutionEnvironment {
    fn emit(&mut self, event_type: String, event_data: Vec<u8>) -> anyhow::Result<()> {
        self.events.push((event_type, event_data));
        Ok(())
    }
}

impl Logging for ExecutionEnvironment {
    fn log(&mut self, output: String) -> anyhow::Result<()> {
        println!("{}", output);
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

impl From<&ExecutionEnvironment> for Changelist {
    fn from(value: &ExecutionEnvironment) -> Self {
        Self {
            account_changes: value.accounts.clone(),
            logs: value.events.clone(),
            storage: value.storage.clone(),
        }
    }
}

pub struct QueryEnvironment {
    storage: SparseMerkleTree,
    state_db: Arc<dyn StateDB>,
}

impl QueryEnvironment {
    pub fn new(storage: SparseMerkleTree, state_db: Arc<dyn StateDB>) -> anyhow::Result<Self> {
        Ok(Self { storage, state_db })
    }

    fn get_account_state(&self, address: Address) -> AccountState {
        self.state_db.account_state(&address)
    }
}

impl Logging for QueryEnvironment {
    fn log(&mut self, _: String) -> anyhow::Result<()> {
        bail!("cannot log in query environment")
    }
}

impl ExecutionContext for QueryEnvironment {
    fn value(&mut self) -> anyhow::Result<u64> {
        bail!("execution context not available in query environment")
    }

    fn block_level(&mut self) -> anyhow::Result<u32> {
        bail!("execution context not available in query environment")
    }

    fn sender(&mut self) -> anyhow::Result<Vec<u8>> {
        bail!("execution context not available in query environment")
    }

    fn network(&mut self) -> anyhow::Result<u32> {
        bail!("execution context not available in query environment")
    }

    fn sender_pk(&mut self) -> anyhow::Result<Vec<u8>> {
        bail!("execution context not available in query environment")
    }
}

impl Event for QueryEnvironment {
    fn emit(&mut self, _: String, _: Vec<u8>) -> anyhow::Result<()> {
        bail!("cannot emit events in query environment")
    }
}

impl Storage for QueryEnvironment {
    fn insert(&mut self, _: Vec<u8>, _: Vec<u8>) -> anyhow::Result<()> {
        bail!("cannot mutate state in query environment")
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        let Ok(data) = self.storage.get(key) else {
            return Ok(None)
        };
        Ok(Some(data))
    }

    fn remove(&mut self, _: Vec<u8>) -> anyhow::Result<bool> {
        bail!("cannot mutate state in query environment")
    }
}

impl Syscall for QueryEnvironment {
    fn block_hash(&mut self, _: u32) -> anyhow::Result<Vec<u8>> {
        bail!("cannot make sys call (block_hash) in query environment")
    }

    fn address_from_pk(&mut self, _: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        bail!("cannot make sys call (address_from_pk) in query environment")
    }

    fn generate_keypair(&mut self) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        bail!("cannot make sys call (generate_keypair) in query environment")
    }

    fn generate_native_address(&mut self, _: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        bail!("cannot make sys call (generate_native_address) in query environment")
    }

    fn sign(&mut self, _: Vec<u8>, _: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        bail!("cannot make sys call (sign) in query environment")
    }

    fn transfer(&mut self, _: Vec<u8>, _: u64) -> anyhow::Result<bool> {
        bail!("cannot make sys call (transfer) in query environment")
    }

    fn reserve(&mut self, _: u64) -> anyhow::Result<bool> {
        bail!("cannot make sys call (reserve) in query environment")
    }

    fn unreserve(&mut self, _: u64) -> anyhow::Result<bool> {
        bail!("cannot make sys call (unreserve) in query environment")
    }

    fn get_free_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .free_balance)
    }

    fn get_nonce(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .nonce)
    }

    fn get_reserve_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64> {
        Ok(self
            .get_account_state(Address::from_slice(address.as_slice()))
            .reserve_balance)
    }
}
