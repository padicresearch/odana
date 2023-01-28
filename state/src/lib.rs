use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use bincode::{Decode, Encode};
use codec::{Decodable, Encodable};
use serde::{Deserialize, Serialize};

use primitive_types::{Address, H256};
use smt::proof::Proof;
use smt::SparseMerkleTree;
use traits::{AppData, StateDB};
use transaction::{NoncePricedTransaction, TransactionsByNonceAndPrice};
use types::account::AccountState;
use types::tx::SignedTransaction;
use types::Hash;

use crate::error::StateError;
use crate::tree::{Op, TrieDB};

mod context;
mod error;
mod persistent;
mod store;
mod tree;

#[derive(Encode, Decode, Clone, Debug)]
pub struct ReadProof {
    proof: Proof,
    root: H256,
}

#[derive(Clone)]
pub struct State {
    trie: Arc<TrieDB<Address, AccountState>>,
    appdata: Arc<TrieDB<AppStateKey, SparseMerkleTree>>,
    path: PathBuf,
    read_only: bool,
}

#[derive(Encode, Decode, Clone, Debug)]
struct AppStateKey(Address, H256);

impl Encodable for AppStateKey {
    fn encode(&self) -> Result<Vec<u8>> {
        bincode::encode_to_vec(self, codec::config()).map_err(|e| anyhow!(e))
    }
}

impl Decodable for AppStateKey {
    fn decode(buf: &[u8]) -> Result<Self> {
        bincode::decode_from_slice(buf, codec::config())
            .map(|(out, _)| out)
            .map_err(|e| anyhow!(e))
    }
}

unsafe impl Sync for State {}

unsafe impl Send for State {}

impl StateDB for State {
    fn nonce(&self, address: &Address) -> u64 {
        self.account_state(address).nonce
    }

    fn account_state(&self, address: &Address) -> AccountState {
        match self.trie.get(address) {
            Ok(Some(account_state)) => account_state,
            _ => AccountState::new(),
        }
    }

    fn balance(&self, address: &Address) -> u64 {
        self.account_state(address).free_balance
    }

    fn credit_balance(&self, address: &Address, amount: u64) -> Result<H256> {
        let mut account_state = self.get_account_state(address)?;
        account_state.free_balance += amount;
        self.trie.put(*address, account_state)?;
        Ok(self.root_hash()?.into())
    }

    fn debit_balance(&self, address: &Address, amount: u64) -> Result<H256> {
        let mut account_state = self.get_account_state(address)?;
        account_state.free_balance -= amount;
        self.trie.put(*address, account_state)?;
        Ok(self.root_hash()?.into())
    }

    fn reset(&self, root: H256) -> Result<()> {
        self.trie.reset(root)
    }

    fn apply_txs(&self, txs: &[SignedTransaction]) -> Result<H256> {
        self.apply_txs(txs)?;
        self.root_hash().map(H256::from)
    }

    fn root(&self) -> Hash {
        self.root_hash().unwrap()
    }

    fn commit(&self) -> Result<()> {
        self.commit()
    }

    fn snapshot(&self) -> Result<Arc<dyn StateDB>> {
        Ok(self.get_sate_at(H256::from(self.root()))?)
    }

    fn state_at(&self, root: H256) -> Result<Arc<dyn StateDB>> {
        Ok(self.get_sate_at(root)?)
    }
}

impl AppData for State {
    fn get_app_data(&self, app_id: Address) -> Result<SparseMerkleTree> {
        let Ok(Some(app_account_state)) = self.trie.get(&app_id) else {
            bail!("app not found")
        };

        let Some(app_root) = app_account_state.root_hash.map(|root| H256::from_slice(&root)) else {
            bail!("app not initialized")
        };

        Ok(self
            .appdata
            .get(&AppStateKey(app_id, app_root))?
            .unwrap_or_else(|| SparseMerkleTree::new()))
    }

    fn set_app_data(&self, app_id: Address, app_data: SparseMerkleTree) -> Result<()> {
        self.appdata
            .put(AppStateKey(app_id, app_data.root()), app_data)
    }

    fn get_app_root(&self, app_id: Address) -> H256 {
        let Ok(Some(app_account_state)) = self.trie.get(&app_id) else {
            return H256::zero()
        };

        let Some(app_root) = app_account_state.root_hash.map(|root| H256::from_slice(&root)) else {
            return H256::zero()
        };
        app_root
    }

    fn get_app_data_at_root(&self, app_id: Address, root: H256) -> Result<SparseMerkleTree> {
        Ok(self
            .appdata
            .get(&AppStateKey(app_id, root))?
            .unwrap_or_else(|| SparseMerkleTree::new()))
    }
}

impl State {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let trie = TrieDB::open(path.join("state").as_path())?;
        let app_state = TrieDB::open(path.join("appdata").as_path())?;
        Ok(Self {
            trie: Arc::new(trie),
            appdata: Arc::new(app_state),
            path: path.clone(),
            read_only: false,
        })
    }

    pub fn apply_txs(&self, txs: &[SignedTransaction]) -> Result<()> {
        let mut accounts: BTreeMap<Address, TransactionsByNonceAndPrice> = BTreeMap::new();
        let mut states: BTreeMap<Address, AccountState> = BTreeMap::new();

        for tx in txs {
            if let std::collections::btree_map::Entry::Vacant(e) = states.entry(tx.from()) {
                let current_state = self.get_account_state(&tx.from())?;
                e.insert(current_state);
            }
            if let std::collections::btree_map::Entry::Vacant(e) = states.entry(tx.to()) {
                let current_state = self.get_account_state(&tx.to())?;
                e.insert(current_state);
            }
            let txs = accounts.entry(tx.from()).or_default();
            txs.insert(NoncePricedTransaction(tx));
        }

        for (_, txs) in accounts {
            for tx in txs {
                self.apply_transaction(tx.0, &mut states)?;
            }
        }

        for (acc, state) in states {
            self.trie.put(acc, state)?;
        }
        Ok(())
    }

    pub fn apply_txs_no_commit(
        &self,
        at_root: H256,
        reward: u64,
        coinbase: Address,
        txs: &[SignedTransaction],
    ) -> Result<Hash> {
        let mut accounts: BTreeMap<Address, TransactionsByNonceAndPrice> = BTreeMap::new();
        let mut states: BTreeMap<Address, AccountState> = BTreeMap::new();

        for tx in txs {
            if let std::collections::btree_map::Entry::Vacant(e) = states.entry(tx.from()) {
                let current_state = self.get_account_state_at_root(&at_root, &tx.from())?;
                e.insert(current_state);
            }
            if let std::collections::btree_map::Entry::Vacant(e) = states.entry(tx.to()) {
                let current_state = self.get_account_state_at_root(&at_root, &tx.to())?;
                e.insert(current_state);
            }
            let txs = accounts.entry(tx.from()).or_default();
            txs.insert(NoncePricedTransaction(tx));
        }

        for (_, txs) in accounts {
            for tx in txs {
                self.apply_transaction(tx.0, &mut states)?;
            }
        }

        let mut batch: Vec<_> = states.into_iter().map(|(k, v)| Op::Put(k, v)).collect();

        let mut coinbase_account_state = self
            .trie
            .get_at_root(&at_root, &coinbase)
            .unwrap_or_default()
            .unwrap_or_default();
        coinbase_account_state.free_balance += reward;
        batch.push(Op::Put(coinbase, coinbase_account_state));

        self.trie
            .apply_non_commit(&at_root, batch)
            .map(|hash| hash.to_fixed_bytes())
    }

    fn apply_transaction(
        &self,
        transaction: &SignedTransaction,
        states: &mut BTreeMap<Address, AccountState>,
    ) -> Result<()> {
        //TODO: verify transaction (probably)

        let mut from_account_state = states
            .get_mut(&transaction.from())
            .ok_or(StateError::AccountNotFound)?;
        let amount = transaction.price() + transaction.fees();
        if from_account_state.free_balance < amount {
            return Err(StateError::InsufficientFunds.into());
        }
        from_account_state.free_balance -= amount;
        let next_nonce = if transaction.nonce() > from_account_state.nonce {
            transaction.nonce() + 1
        } else {
            from_account_state.nonce + 1
        };
        from_account_state.nonce = next_nonce;

        let mut to_account_state = states
            .get_mut(&transaction.to())
            .ok_or(StateError::AccountNotFound)?;
        to_account_state.free_balance += transaction.price();

        Ok(())
    }

    fn commit(&self) -> Result<()> {
        self.trie.commit(!self.read_only)?;
        Ok(())
    }

    pub fn check_transaction(&self, _transaction: &SignedTransaction) -> Result<()> {
        Ok(())
    }

    fn get_account_state(&self, address: &Address) -> Result<AccountState> {
        match self.trie.get(address) {
            Ok(Some(account_state)) => Ok(account_state),
            _ => Ok(AccountState::new()),
        }
    }

    fn get_account_state_at_root(&self, at_root: &H256, address: &Address) -> Result<AccountState> {
        match self.trie.get_at_root(at_root, address) {
            Ok(Some(account_state)) => Ok(account_state),
            _ => Ok(AccountState::new()),
        }
    }

    pub fn get_sate_at(&self, root: H256) -> Result<Arc<Self>> {
        let mut path = PathBuf::new();
        path.push(self.path.as_path());
        let trie = TrieDB::open(path.join("state").as_path())?;
        let appdata = TrieDB::open(path.join("appdata").as_path())?;
        Ok(Arc::new(State {
            trie: Arc::new(trie),
            appdata: Arc::new(appdata),
            path: self.path.clone(),
            read_only: true,
        }))
    }

    fn get_account_state_with_proof(&self, address: &Address) -> Result<(AccountState, ReadProof)> {
        let (account_state, proof) = self.trie.get_with_proof(address)?;
        let root = self.trie.root()?;
        Ok((account_state, ReadProof { proof, root }))
    }

    pub fn checkpoint<P: AsRef<Path>>(&self, _path: P) -> Result<Self> {
        unimplemented!()
    }

    pub fn root_hash(&self) -> Result<Hash> {
        self.trie.root().map(|root| root.to_fixed_bytes())
    }
}

pub trait MorphCheckPoint {
    fn checkpoint(&self) -> State;
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use account::create_account;
    use types::network::Network;

    use super::*;

    #[test]
    fn test_morph() {
        let temp = TempDir::new("state").unwrap();
        let mut path = PathBuf::new();
        path.push(temp.path());
        let state = State::new(&path).unwrap();
        let alice = create_account(Network::Testnet);
        let _bob = create_account(Network::Testnet);
        let _jake = create_account(Network::Testnet);
        println!(
            "{}",
            state.credit_balance(&alice.address, 1_000_000).unwrap()
        );
        state.commit().unwrap();
        println!(
            "{}",
            state.credit_balance(&alice.address, 1_000_000).unwrap()
        );
        state.commit().unwrap();
        println!(
            "{}",
            state.credit_balance(&alice.address, 1_000_000).unwrap()
        );
    }
}
