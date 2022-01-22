use crate::error::MorphError;
use crate::kv::Schema;
use crate::store::{
    AccountMetadataStorage, AccountStateStorage, HistoryIKey, HistoryIValue, HistoryStorage,
};
use crate::{
    get_operations, AccountState, Hash, Morph, MorphOperation, MorphStorageKV, GENESIS_ROOT,
};
use anyhow::{Error, Result};
use codec::{Decoder, Encoder};
use primitive_types::H160;
use rocksdb::{BoundColumnFamily, ColumnFamily, Snapshot, DB};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tiny_keccak::Hasher;
use traits::StateDB;
use types::tx::Transaction;

pub struct MorphIntermediate<'a> {
    cfs: Arc<BTreeMap<&'static str, Arc<BoundColumnFamily<'a>>>>,
    snapshot: Arc<Snapshot<'a>>,
    account_state: RwLock<BTreeMap<H160, AccountState>>,
    applied_txs: RwLock<BTreeSet<Transaction>>,
    current_root: RwLock<Hash>,
}

impl<'a> MorphIntermediate<'a> {
    pub fn new(
        root: Hash,
        cfs: BTreeMap<&'static str, Arc<BoundColumnFamily<'a>>>,
        snapshot: Snapshot<'a>,
    ) -> Self {
        Self {
            cfs: Arc::new(cfs),
            snapshot: Arc::new(snapshot),
            account_state: Default::default(),
            applied_txs: Default::default(),
            current_root: RwLock::new(root),
        }
    }
}

unsafe impl<'a> Sync for MorphIntermediate<'a> {}

unsafe impl<'a> Send for MorphIntermediate<'a> {}

impl<'a> StateDB for MorphIntermediate<'a> {
    fn nonce(&self, address: &H160) -> u64 {
        self.account_state(address).nonce
    }

    fn account_state(&self, address: &H160) -> AccountState {
        self.account_state(address)
    }

    fn balance(&self, address: &H160) -> u128 {
        self.account_state(address).free_balance
    }

    fn credit_balance(&self, address: &H160, amount: u128) -> Result<Hash> {
        let action = MorphOperation::CreditBalance {
            account: *address,
            amount,
            tx_hash: [0; 32],
        };

        let mut current_root = self
            .current_root
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut account_state = self
            .account_state
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        //Todo: Add coinbase transaction
        self.apply_op(&action, &mut current_root, &mut account_state)
    }

    fn debit_balance(&self, address: &H160, amount: u128) -> Result<Hash> {
        let action = MorphOperation::DebitBalance {
            account: *address,
            amount,
            tx_hash: [0; 32],
        };

        let mut current_root = self
            .current_root
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut account_state = self
            .account_state
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        //Todo: Add coinbase transaction
        self.apply_op(&action, &mut current_root, &mut account_state)
    }
}

impl<'a> MorphIntermediate<'a> {
    pub fn apply_transaction(&self, tx: &Transaction) -> Result<Hash> {
        let mut current_root = self
            .current_root
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut account_state = self
            .account_state
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut applied_txs = self
            .applied_txs
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        anyhow::ensure!(
            applied_txs.contains(tx) == false,
            MorphError::TransactionAlreadyApplied
        );
        for action in get_operations(tx).iter() {
            let _ = self.apply_op(action, &mut current_root, &mut account_state)?;
        }
        applied_txs.insert(tx.clone());
        Ok(*current_root)
    }

    pub fn root(&self) -> Hash {
        let mut current_root = self
            .current_root
            .read()
            .map_err(|e| anyhow::anyhow!("{}", e)).unwrap();
        *current_root
    }

    fn apply_op(
        &self,
        action: &MorphOperation,
        current_root: &mut Hash,
        account_states: &mut BTreeMap<H160, AccountState>,
    ) -> Result<Hash> {
        let new_account_state = self.apply_operation(action)?;
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(current_root);
        sha3.update(&action.encode()?);
        sha3.update(&new_account_state.encode()?);
        let mut new_root = [0; 32];
        sha3.finalize(&mut new_root);
        *current_root = new_root.clone();
        account_states.insert(action.get_address(), new_account_state);
        Ok(new_root)
    }

    fn origin_current_root(&self) -> Hash {
        let column_name = HistoryStorage::column();
        let cf = match self
            .cfs
            .get(column_name)
            .ok_or(MorphError::ColumnFamilyMissing(column_name))
        {
            Ok(v) => v.clone(),
            Err(_) => return GENESIS_ROOT,
        };
        let encoded_key = match HistoryIKey::Root.encode() {
            Ok(k) => k,
            Err(_) => return GENESIS_ROOT,
        };
        let value = match self.snapshot.get_cf(&cf, encoded_key) {
            Ok(v) => v,
            Err(_) => return GENESIS_ROOT,
        };
        match value {
            None => {
                return GENESIS_ROOT;
            }
            Some(value) => HistoryIValue::decode(&value)
                .map(|v| v.root)
                .unwrap_or(GENESIS_ROOT),
        }
    }

    fn apply_operation(&self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::DebitBalance {
                account, amount, ..
            } => {
                let mut account_state = self.get_account(account)?;
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance {
                account, amount, ..
            } => {
                let mut account_state = self.get_account(account)?;
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce, .. } => {
                let mut account_state = self.get_account(account)?;
                if *nonce <= account_state.nonce {
                    return Err(MorphError::NonceIsLessThanCurrent.into());
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }
    fn account_state_storage_get(&self, address: &H160) -> Result<Option<AccountState>> {
        let column_name = AccountStateStorage::column();
        let cf = self
            .cfs
            .get(column_name)
            .ok_or(MorphError::ColumnFamilyMissing(column_name))?;
        let encoded_key = address.encode()?;
        let value = self.snapshot.get_cf(cf, encoded_key)?;
        match value {
            None => Ok(None),
            Some(value) => Ok(Some(AccountState::decode(&value)?)),
        }
    }
    fn get_account(&self, account_id: &H160) -> Result<AccountState> {
        let mut account_state = self
            .account_state
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        if let Some(state) = account_state.get(account_id) {
            return Ok(state.clone());
        }
        if let Ok(Some(state)) = self.account_state_storage_get(account_id) {
            return Ok(state);
        }
        Ok(AccountState::default())
    }

    pub fn account_nonce(&self, address: &H160) -> u64 {
        self.get_account(address).unwrap_or_default().nonce
    }

    pub fn account_state(&self, address: &H160) -> AccountState {
        self.get_account(address).unwrap_or_default()
    }
}
