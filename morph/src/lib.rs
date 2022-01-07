use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use account::get_address_from_pub_key;
use codec::impl_codec;
use codec::{Codec, Decoder, Encoder};
use storage::{KVEntry, KVStore};
use types::tx::{Transaction, TransactionKind};
use types::{BlockHash, TxHash};

use crate::error::MorphError;
use crate::logdb::{HistoryLog, LogData};
use crate::snapshot::MorphSnapshot;
use primitive_types::H160;
use traits::StateDB;
use types::account::AccountState;

mod error;
mod logdb;
mod snapshot;

type Hash = [u8; 32];

const GENESIS_ROOT: [u8; 32] = [0; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProofElement {
    Operation { op: MorphOperation, index: u64 },
    History { index: u64 },
}

pub type MorphStorageKV = dyn KVStore<Morph> + Send + Sync;

#[derive(Clone)]
pub struct Morph {
    kv: Arc<MorphStorageKV>,
    history_log: Arc<HistoryLog>,
}

impl KVEntry for Morph {
    type Key = H160;
    type Value = AccountState;

    fn column() -> &'static str {
        "state"
    }
}

impl StateDB for Morph {
    fn account_nonce(&self, address: &H160) -> u64 {
        match self
            .get_account_state(address)
            .map(|account_state| account_state.map(|state| state.nonce as u64))
        {
            Ok(Some(nonce)) => nonce,
            _ => 0,
        }
    }

    fn account_state(&self, address: &H160) -> AccountState {
        match self.get_account_state(address) {
            Ok(Some(state)) => state,
            _ => AccountState::default(),
        }
    }
}

impl Morph {
    pub fn new(kv: Arc<MorphStorageKV>, history_log: Arc<HistoryLog>) -> Result<Self> {
        let mut morph = Self { kv, history_log };
        Ok(morph)
    }

    pub fn apply_transaction(&mut self, transaction: Transaction) -> Result<()> {
        //TODO: verify transaction (probably)
        for action in get_operations(&transaction) {
            let mut new_account_state = self.apply_action(&action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            let current_root = match self.history_log.last_history() {
                None => {
                    if self.history_log.len() > 0 {
                        return Err(MorphError::ValidationFailedRootNotValid.into());
                    }
                    GENESIS_ROOT
                }
                Some(root) => root,
            };
            sha3.update(&current_root);
            sha3.update(&action.encode()?);
            sha3.update(&new_account_state.encode()?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);
            self.history_log
                .append(LogData::new(action.clone(), new_root));
            self.kv.put(action.get_address(), new_account_state)?;
        }
        Ok(())
    }

    //fn commit(&self, new_root : )

    pub fn check_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        Ok(())
    }

    fn apply_action(&mut self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::DebitBalance {
                account, amount, ..
            } => {
                let mut account_state = self.kv.get(account)?.ok_or(MorphError::AccountNotFound)?;
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance {
                account, amount, ..
            } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce, .. } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                if *nonce <= account_state.nonce {
                    return Err(MorphError::NonceIsLessThanCurrent.into());
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, address: &H160) -> Result<Option<AccountState>> {
        self.kv.get(address)
    }

    fn proof(&self, address: H160) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for res in self.history_log.iter_operations()? {
            let (idx, op) = res?;
            let account = op.get_address();
            if account == address {
                proof.push(ProofElement::Operation {
                    op: op.clone(),
                    index: idx as u64,
                })
            } else {
                proof.push(ProofElement::History { index: idx as u64 });
            }
        }
        Ok(proof)
    }

    pub fn create_snapshot(&self) -> Result<MorphSnapshot> {
        MorphSnapshot::new(self)
    }

    pub fn commit_snapshot(&self, snapshot: MorphSnapshot) -> Result<()> {
        Ok(())
    }

    fn compact_proof(&self, address: H160) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for res in self.history_log.iter_operations()? {
            let (idx, op) = res?;
            let op_address = op.get_address();
            if op_address == address {
                proof.push(ProofElement::Operation {
                    op: op.clone(),
                    index: idx as u64,
                })
            } else {
                if let Some(ProofElement::History { .. }) = proof.last() {
                    let d = proof.len() - 1;
                    std::mem::swap(
                        proof
                            .get_mut(d)
                            .ok_or(anyhow::anyhow!("poof index not found"))?,
                        &mut ProofElement::History { index: idx as u64 },
                    )
                    //proof.push();
                } else {
                    proof.push(ProofElement::History { index: idx as u64 });
                }
            }
        }
        Ok(proof)
    }

    pub fn root_hash(&self) -> Option<Hash> {
        self.history_log.last_history()
    }
}

// fn hash_op(prev_hash : &Hash, op : &MorphOperation) -> Hash {
//
// }

pub fn validate_account_state(
    proof: &Vec<ProofElement>,
    morph: &Morph,
) -> Result<(AccountState, u64), MorphError> {
    let mut account_state: AccountState = AccountState::default();
    let mut history: Vec<[u8; 32]> = vec![];
    let mut last_valid_seq = 0;
    for el in proof {
        match el {
            ProofElement::Operation { op, index: seq } => {
                match op {
                    MorphOperation::DebitBalance { amount, .. } => {
                        account_state.free_balance =
                            account_state.free_balance.saturating_sub(*amount);
                    }
                    MorphOperation::CreditBalance { amount, .. } => {
                        account_state.free_balance =
                            account_state.free_balance.saturating_add(*amount);
                    }
                    MorphOperation::UpdateNonce { nonce, .. } => {
                        account_state.nonce = *nonce;
                    }
                }
                let prev_root = history.last().unwrap_or(&GENESIS_ROOT);
                let mut sha3 = tiny_keccak::Sha3::v256();
                sha3.update(prev_root);
                sha3.update(&op.encode()?);
                sha3.update(&account_state.encode()?);
                let mut new_root = [0; 32];
                sha3.finalize(&mut new_root);
                let valid_history_hash = morph
                    .history_log
                    .get_root_at(*seq)?
                    .ok_or((MorphError::ValidationFailedHistoryNotFound))?;
                if new_root != valid_history_hash {
                    return Err(MorphError::ValidationFailedRootNotValid);
                }
                history.push(new_root);
                last_valid_seq = *seq
            }
            ProofElement::History { index: seq } => {
                history.push(
                    morph
                        .history_log
                        .get_root_at(*seq)?
                        .ok_or(MorphError::ValidationFailedHistoryNotFound)?,
                );
            }
        }
    }

    Ok((account_state, last_valid_seq))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MorphOperation {
    DebitBalance {
        account: H160,
        amount: u128,
        tx_hash: Hash,
    },
    CreditBalance {
        account: H160,
        amount: u128,
        tx_hash: Hash,
    },
    UpdateNonce {
        account: H160,
        nonce: u64,
        tx_hash: Hash,
    },
}

impl MorphOperation {
    fn get_address(&self) -> H160 {
        match self {
            MorphOperation::DebitBalance { account, .. } => *account,
            MorphOperation::CreditBalance { account, .. } => *account,
            MorphOperation::UpdateNonce { account, .. } => *account,
        }
    }
}

pub fn get_operations(tx: &Transaction) -> Vec<MorphOperation> {
    let mut ops = Vec::new();
    let tx_hash = tx.hash();
    match tx.kind() {
        TransactionKind::Transfer {
            from,
            to,
            amount,
            fee,
            ..
        } => {
            ops.push(MorphOperation::DebitBalance {
                account: get_address_from_pub_key(from),
                amount: *amount + *fee,
                tx_hash,
            });
            ops.push(MorphOperation::CreditBalance {
                account: get_address_from_pub_key(to),
                amount: *amount,
                tx_hash,
            });
            ops.push(MorphOperation::UpdateNonce {
                account: get_address_from_pub_key(from),
                nonce: tx.nonce(),
                tx_hash,
            });
        }
        TransactionKind::Coinbase { amount, miner, .. } => {
            ops.push(MorphOperation::CreditBalance {
                account: get_address_from_pub_key(miner),
                amount: *amount,
                tx_hash,
            });
        }
    }
    ops
}
impl_codec!(MorphOperation);

pub trait MorphCheckPoint {
    fn checkpoint(&self) -> Morph;
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use commitlog::{CommitLog, LogOptions};
    use tempdir::TempDir;

    use account::create_account;
    use storage::memstore::MemStore;
    use storage::sleddb::SledDB;
    use transaction::make_sign_transaction;

    use super::*;

    #[test]
    fn test_morph() {
        let tmp_dir = TempDir::new("maindb").unwrap();
        let database = Arc::new(SledDB::new(tmp_dir.path()).unwrap());
        let tmp_dir = TempDir::new("history").unwrap();
        let commit_log = Arc::new(RwLock::new(
            CommitLog::new(LogOptions::new(tmp_dir.path())).unwrap(),
        ));
        let mut morph = Morph::new(
            database.clone(),
            Arc::new(HistoryLog::new(commit_log, database).unwrap()),
        )
            .unwrap();
        let alice = create_account();
        let bob = create_account();
        let jake = create_account();

        morph.apply_transaction(
            make_sign_transaction(
                &alice,
                1,
                TransactionKind::Coinbase {
                    miner: alice.pub_key,
                    block_hash: bob.pub_key,
                    amount: 10000000,
                },
            )
                .unwrap(),
        );
        for i in 0..1000 {
            let amount = i as u128 * 10;
            assert!(morph
                .apply_transaction(
                    make_sign_transaction(
                        &alice,
                        i + 2000,
                        TransactionKind::Transfer {
                            from: alice.pub_key,
                            to: bob.pub_key,
                            amount,
                            fee: (amount as f64 * 0.01) as u128,
                        },
                    )
                        .unwrap()
                )
                .is_ok());
            if i % 100 == 0 {
                assert_eq!(
                    validate_account_state(&morph.compact_proof(alice.address).unwrap(), &morph)
                        .unwrap()
                        .0,
                    morph.get_account_state(&alice.address).unwrap().unwrap()
                );
                assert_eq!(
                    validate_account_state(&morph.compact_proof(bob.address).unwrap(), &morph)
                        .unwrap()
                        .0,
                    morph.get_account_state(&bob.address).unwrap().unwrap()
                );
            }
        }

        //assert!()
    }
}
