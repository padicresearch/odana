use crate::error::MorphError;
use crate::{get_operations, AccountState, Hash, Morph, MorphOperation, MorphStorageKV, GENESIS_ROOT};
use anyhow::{Result, Error};
use codec::{Encoder, Decoder};
use primitive_types::H160;
use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::sync::Arc;
use tiny_keccak::Hasher;
use types::tx::Transaction;
use types::TxHash;
use rocksdb::{Snapshot, DB, ColumnFamily};
use crate::store::{HistoryStorage, AccountStateStorage, AccountMetadataStorage, HistoryIKey, HistoryIValue};
use crate::kv::Schema;
use traits::StateDB;

pub struct MorphIntermediate<'a> {
    cfs: BTreeMap<&'static str, &'a ColumnFamily>,
    snapshot: Snapshot<'a>,
    account_state: BTreeMap<H160, AccountState>,
    applied_txs: BTreeSet<Transaction>,
    current_root: Option<Hash>,
}

impl<'a> MorphIntermediate<'a> {
    pub fn new(cfs: BTreeMap<&'static str, &'a ColumnFamily>, snapshot: Snapshot<'a>) -> Self {
        Self {
            cfs,
            snapshot,
            account_state: Default::default(),
            applied_txs: Default::default(),
            current_root: None,
        }
    }
}


impl<'a> MorphIntermediate<'a> {
    pub fn apply_transaction(&mut self, tx: &Transaction) -> Result<()> {
        anyhow::ensure!(
            self.applied_txs.contains(tx) == false,
            MorphError::TransactionAlreadyApplied
        );
        for action in get_operations(tx).iter() {
            let new_account_state = self.apply_action(action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            sha3.update(&self.root());
            sha3.update(&action.encode()?);
            sha3.update(&new_account_state.encode()?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);
            self.current_root = Some(new_root);
            self.account_state
                .insert(action.get_address(), new_account_state);
        }
        self.applied_txs.insert(tx.clone());
        Ok(())
    }

    pub(crate) fn root(&self) -> Hash {
        self.current_root.unwrap_or(self.origin_current_root())
    }

    fn origin_current_root(&self) -> Hash {
        let column_name = HistoryStorage::column();
        let cf = match self.cfs.get(column_name).ok_or(MorphError::ColumnFamilyMissing(column_name)) {
            Ok(v) => {
                *v
            }
            Err(_) => {
                return GENESIS_ROOT
            }
        };
        let encoded_key = match HistoryIKey::Root.encode() {
            Ok(k) => {
                k
            }
            Err(_) => {
                return GENESIS_ROOT
            }
        };
        let value = match self.snapshot.get_cf(cf, encoded_key) {
            Ok(v) => { v }
            Err(_) => {
                return GENESIS_ROOT
            }
        };
        match value {
            None => {
                return GENESIS_ROOT;
            }
            Some(value) => {
                HistoryIValue::decode(&value).map(|v| v.root).unwrap_or(GENESIS_ROOT)
            }
        }
    }

    fn apply_action(&mut self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::DebitBalance {
                account, amount, ..
            } => {
                let mut account_state = self.get_account(account);
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance {
                account, amount, ..
            } => {
                let mut account_state = self.get_account(account);
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce, .. } => {
                let mut account_state = self.get_account(account);
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
        let cf = *self.cfs.get(column_name).ok_or(MorphError::ColumnFamilyMissing(column_name))?;
        let encoded_key = address.encode()?;
        let value = self.snapshot.get_cf(cf, encoded_key)?;
        match value {
            None => {
                Ok(None)
            }
            Some(value) => {
                Ok(Some(AccountState::decode(&value)?))
            }
        }
    }
    fn get_account(&self, account_id: &H160) -> AccountState {
        if let Some(state) = self.account_state.get(account_id) {
            return state.clone();
        }
        if let Ok(Some(state)) = self.account_state_storage_get(account_id) {
            return state;
        }
        AccountState::default()
    }

    fn account_nonce(&self, address: &H160) -> u64 {
        self.get_account(address).nonce
    }

    pub(crate) fn account_state(&self, address: &H160) -> AccountState {
        self.get_account(address)
    }
}
