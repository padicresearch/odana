use crate::{Hash, MorphOperation, MorphStorageKV, AccountId, AccountState, get_operations, Morph};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use anyhow::Result;
use crate::error::Error;
use tiny_keccak::Hasher;
use transaction::Transaction;
use types::TxHash;
use codec::Encoder;

pub struct MorphSnapshot {
    origin_root : Hash,
    origin_index : u64,
    current_root : Hash,
    current_seq : u64,
    roots: Vec<Hash>,
    log : Vec<MorphOperation>,
    kv : Arc<MorphStorageKV>,
    applied_txs : HashSet<TxHash>,
    account_state : HashMap<AccountId, AccountState>,
}

impl MorphSnapshot {
    pub fn new( morph : &Morph) -> Result<Self> {
        let root = morph.history_log.last_history().ok_or(Error::SnapshotCreationErrorRootNotFound)?;
        let index = morph.history_log.len() - 1;
        let roots = vec![root];
        let log = morph.history_log.last_op().map(|op| vec![op.clone()]).unwrap_or_default();
        Ok(Self {
            origin_root:root ,
            origin_index: index,
            current_root: root,
            current_seq: index,
            roots,
            log,
            kv: morph.kv.clone(),
            applied_txs: Default::default(),
            account_state: Default::default()
        })
    }
}


impl MorphSnapshot {
    pub fn apply_transaction(&mut self, tx : &Transaction) -> Result<()> {
        let tx_hash = tx.id();
        anyhow::ensure!(self.applied_txs.contains(&tx_hash) == false, Error::TransactionAlreadyApplied);
        for action in get_operations(tx).iter() {
            let new_account_state = self.apply_action(action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            sha3.update(&self.current_root);
            sha3.update(&action.encode()?);
            sha3.update(&new_account_state.encode()?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);
            self.current_root = new_root;
            self.log.push(action.clone());
            self.account_state.insert(action.get_account_id(), new_account_state);
        }
        self.applied_txs.insert(tx.id());
        Ok(())
    }

    fn apply_action(&mut self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::DebitBalance { account, amount, .. } => {
                let mut account_state = self.get_account(account);
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance { account, amount, .. } => {
                let mut account_state = self.get_account(account);
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce,.. } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                if *nonce <= account_state.nonce {
                    return Err(Error::NonceIsLessThanCurrent.into())
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account(&self, account_id : &AccountId) -> AccountState {
        if let Some(state) = self.account_state.get(account_id){
            return state.clone()
        }
        if let Ok(Some(state)) = self.kv.get(account_id) {
            return state
        }
        AccountState::default()
    }
}