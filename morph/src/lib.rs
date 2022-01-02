mod error;
mod logdb;
mod snapshot;

use crate::error::Error;
use crate::logdb::{HistoryLog, LogData};
use crate::snapshot::MorphSnapshot;
use account::{Account, TREASURY_ACCOUNT_PK};
use anyhow::Result;
use chrono::Utc;
use codec::impl_codec;
use codec::{Codec, Decoder, Encoder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use storage::{KVEntry, KVStore};
use tiny_keccak::Hasher;
use transaction::{verify_signed_transaction, Transaction, TransactionKind};
use types::{AccountId, BlockHash, TxHash};

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
    type Key = AccountId;
    type Value = AccountState;

    fn column() -> &'static str {
        "account_balances"
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
                        return Err(Error::ValidationFailedRootNotValid.into());
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
            self.kv.put(action.get_account_id(), new_account_state)?;
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
                let mut account_state = self.kv.get(account)?.ok_or(Error::AccountNotFound)?;
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
                    return Err(Error::NonceIsLessThanCurrent.into());
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, account_id: &Hash) -> Result<Option<AccountState>> {
        self.kv.get(account_id)
    }

    fn proof(&self, account_id: AccountId) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for res in self.history_log.iter_operations()? {
            let (idx, op) = res?;
            let account = op.get_account_id();
            if account == account_id {
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

    fn compact_proof(&self, account_id: AccountId) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for res in self.history_log.iter_operations()? {
            let (idx, op) = res?;
            let account = op.get_account_id();
            if account == account_id {
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
) -> Result<(AccountState, u64), Error> {
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
                    .ok_or((Error::ValidationFailedHistoryNotFound))?;
                if new_root != valid_history_hash {
                    return Err(Error::ValidationFailedRootNotValid);
                }
                history.push(new_root);
                last_valid_seq = *seq
            }
            ProofElement::History { index: seq } => {
                history.push(
                    morph
                        .history_log
                        .get_root_at(*seq)?
                        .ok_or(Error::ValidationFailedHistoryNotFound)?,
                );
            }
        }
    }

    Ok((account_state, last_valid_seq))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MorphOperation {
    DebitBalance {
        account: AccountId,
        amount: u128,
        tx_hash: Hash,
    },
    CreditBalance {
        account: AccountId,
        amount: u128,
        tx_hash: Hash,
    },
    UpdateNonce {
        account: AccountId,
        nonce: u32,
        tx_hash: Hash,
    },
}

impl MorphOperation {
    fn get_account_id(&self) -> AccountId {
        match self {
            MorphOperation::DebitBalance { account, .. } => *account,
            MorphOperation::CreditBalance { account, .. } => *account,
            MorphOperation::UpdateNonce { account, .. } => *account,
        }
    }
}

pub fn get_operations(tx: &Transaction) -> Vec<MorphOperation> {
    let mut ops = Vec::new();
    let tx_hash = tx.id();
    match tx.kind() {
        TransactionKind::Transfer {
            from, to, amount, ..
        } => {
            ops.push(MorphOperation::DebitBalance {
                account: *from,
                amount: *amount,
                tx_hash,
            });
            ops.push(MorphOperation::CreditBalance {
                account: *to,
                amount: *amount,
                tx_hash,
            });
            ops.push(MorphOperation::UpdateNonce {
                account: *from,
                nonce: tx.nonce_u32(),
                tx_hash,
            });
        }
        TransactionKind::Coinbase { amount, miner, .. } => {
            ops.push(MorphOperation::CreditBalance {
                account: *miner,
                amount: *amount,
                tx_hash,
            });
        }
    }
    ops
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    free_balance: u128,
    reserve_balance: u128,
    nonce: u32,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            free_balance: 0,
            reserve_balance: 0,
            nonce: 0,
        }
    }
}

impl_codec!(AccountState);
impl_codec!(MorphOperation);

#[cfg(test)]
mod tests {
    use super::*;
    use account::create_account;
    use commitlog::{CommitLog, LogOptions};
    use std::sync::RwLock;
    use storage::memstore::MemStore;
    use storage::sleddb::SledDB;
    use tempdir::TempDir;
    use transaction::make_sign_transaction;

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
            assert!(morph
                .apply_transaction(
                    make_sign_transaction(
                        &alice,
                        i + 2000,
                        TransactionKind::Transfer {
                            from: alice.pub_key,
                            to: bob.pub_key,
                            amount: i as u128 * 10,
                        },
                    )
                        .unwrap()
                )
                .is_ok());
            if i % 100 == 0 {
                assert_eq!(
                    validate_account_state(&morph.compact_proof(alice.pub_key).unwrap(), &morph)
                        .unwrap()
                        .0,
                    morph.get_account_state(&alice.pub_key).unwrap().unwrap()
                );
                assert_eq!(
                    validate_account_state(&morph.compact_proof(bob.pub_key).unwrap(), &morph)
                        .unwrap()
                        .0,
                    morph.get_account_state(&bob.pub_key).unwrap().unwrap()
                );
            }
        }

        //assert!()
    }
}
