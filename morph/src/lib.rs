use std::collections::{BTreeMap, HashMap};
use std::option::Option::Some;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Error, Result};
use chrono::Utc;
use rocksdb::checkpoint::Checkpoint;
use rocksdb::ColumnFamily;
use serde::{Deserialize, Serialize};
use tiny_keccak::Hasher;

use account::get_address_from_pub_key;
use codec::{Codec, Decoder, Encoder};
use codec::impl_codec;
use primitive_types::{H160, H256};
use storage::{KVEntry, KVStore};
use traits::StateDB;
use types::{BlockHash, TxHash};
use types::account::AccountState;
use types::Hash;
use types::tx::{Transaction, TransactionKind};

use crate::error::MorphError;
//use crate::snapshot::MorphSnapshot;
use crate::kv::Schema;
use crate::logdb::{HistoryLog, LogData};
use crate::snapshot::MorphIntermediate;
use crate::store::{
    AccountMetadataStorage, AccountStateStorage, column_families, default_db_opts,
    HistorySequenceStorage, HistoryStorage,
};

mod error;
mod kv;
mod logdb;
mod snapshot;
mod store;

const GENESIS_ROOT: [u8; 32] = [0; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProofItem {
    op: MorphOperation,
    parent_root: Hash,
    root: Hash,
}
pub type MorphStorageKV = dyn KVStore<Morph> + Send + Sync;

#[derive(Clone)]
pub struct Morph {
    db: Arc<rocksdb::DB>,
    history_storage: Arc<HistoryStorage>,
    account_state_storage: Arc<AccountStateStorage>,
    account_meta_storage: Arc<AccountMetadataStorage>,
}

impl StateDB for Morph {
    fn nonce(&self, address: &H160) -> u64 {
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
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(
            &default_db_opts(),
            path,
            column_families(),
        )?);
        let morph = Self {
            db: db.clone(),
            history_storage: Arc::new(HistoryStorage::new(db.clone())),
            account_state_storage: Arc::new(AccountStateStorage::new(db.clone())),
            account_meta_storage: Arc::new(AccountMetadataStorage::new(db.clone())),
        };
        Ok(morph)
    }

    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = Arc::new(rocksdb::DB::open_cf_for_read_only(
            &default_db_opts(),
            path,
            column_families(),
            false,
        )?);
        let morph = Self {
            db: db.clone(),
            history_storage: Arc::new(HistoryStorage::new(db.clone())),
            account_state_storage: Arc::new(AccountStateStorage::new(db.clone())),
            account_meta_storage: Arc::new(AccountMetadataStorage::new(db.clone())),
        };
        Ok(morph)
    }

    pub fn apply_transaction(&self, transaction: Transaction) -> Result<()> {
        //TODO: verify transaction (probably)
        for action in get_operations(&transaction) {
            let mut new_account_state = self.apply_action(&action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            let current_root = self.history_storage.root_hash()?;
            sha3.update(&current_root);
            sha3.update(&action.encode()?);
            sha3.update(&new_account_state.encode()?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);

            let sender = action.get_address();
            let history_value = self.history_storage.append(new_root, action)?;
            self.account_meta_storage.put(sender, history_value.root)?;
            self.account_state_storage.put(sender, new_account_state)?;
        }
        Ok(())
    }

    //fn commit(&self, new_root : )

    pub fn check_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        Ok(())
    }

    fn apply_action(&self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::DebitBalance {
                account, amount, ..
            } => {
                let mut account_state = self
                    .get_account_state(account)?
                    .ok_or(MorphError::AccountNotFound)?;
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance {
                account, amount, ..
            } => {
                let mut account_state = self.get_account_state(account)?.unwrap_or_default();
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce, .. } => {
                let mut account_state = self.get_account_state(account)?.unwrap_or_default();
                if *nonce <= account_state.nonce {
                    return Err(MorphError::NonceIsLessThanCurrent.into());
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, address: &H160) -> Result<Option<AccountState>> {
        self.account_state_storage.get(address)
    }

    fn get_account_state_with_proof(
        &self,
        address: &H160,
        full_walk: bool,
    ) -> Result<(Option<AccountState>, Vec<ProofItem>)> {
        Ok((
            self.account_state_storage.get(address)?,
            if full_walk {
                self.account_state_full_proof(address.clone())?
            } else {
                self.account_state_partial_proof(address.clone())?
            },
        ))
    }

    fn account_state_full_proof(&self, address: H160) -> Result<Vec<ProofItem>> {
        let mut proof = Vec::new();
        for his in self.history_storage.address_history(address)? {
            proof.push(ProofItem {
                op: his.operation,
                parent_root: his.parent_root,
                root: his.root,
            });
        }
        Ok(proof)
    }

    fn account_state_partial_proof(&self, address: H160) -> Result<Vec<ProofItem>> {
        let mut proof = Vec::new();
        let roots = match self.account_meta_storage.get(&address)? {
            None => return Ok(proof),
            Some(roots) => self.history_storage.multi_get(roots)?,
        };
        let mut roots_iter = roots.iter();
        while let Some(Some(his)) = roots_iter.next() {
            proof.push(ProofItem {
                op: his.operation.clone(),
                parent_root: his.parent_root,
                root: his.root,
            });
        }
        Ok(proof)
    }

    pub fn checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<Self> {
        let checkpoint = Checkpoint::new(&self.db)?;
        checkpoint.create_checkpoint(path.as_ref())?;
        Ok(Self::new(path.as_ref())?)
    }

    pub fn intermediate(&self) -> Result<MorphIntermediate> {
        let cfs = vec![AccountStateStorage::column(), HistoryStorage::column()];
        let cfs: Result<BTreeMap<_, _>, _> = cfs
            .iter()
            .map(|name| {
                self.db
                    .cf_handle(*name)
                    .ok_or(MorphError::ColumnFamilyMissing(name))
                    .map(|cf| (*name, cf))
            })
            .collect();
        let snapshot = self.db.snapshot();
        Ok(MorphIntermediate::new(
            self.history_storage.root_hash()?,
            cfs?,
            snapshot,
        ))
    }
    pub fn root_hash(&self) -> Option<Hash> {
        match self.history_storage.root_hash() {
            Ok(root) => Some(root),
            Err(_) => None,
        }
    }
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
    use std::time::Instant;

    use commitlog::{CommitLog, LogOptions};
    use tempdir::TempDir;

    use account::create_account;
    use storage::memstore::MemStore;
    use storage::sleddb::SledDB;
    use transaction::make_sign_transaction;

    use super::*;

    #[test]
    fn test_morph() {
        let base = std::env::var("TEST_DIR").unwrap();
        fs_extra::dir::remove(format!("{}/state", base)).unwrap();
        let mut morph = Morph::new(format!("{}/state", base)).unwrap();
        let alice = create_account();
        let bob = create_account();
        let jake = create_account();

        morph
            .apply_transaction(
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
            )
            .unwrap();
        for i in 0..100 {
            let amount = 100;
            assert!(morph
                .apply_transaction(
                    make_sign_transaction(
                        &alice,
                        i + 1,
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
        }

        let checkpoint_1 = morph
            .checkpoint(format!("{}/state/{}", base, hex::encode(&[1_u8; 32])))
            .unwrap();
        let mut intermediate = morph.intermediate().unwrap();
        for i in 0..100 {
            let amount = 100;
            assert!(checkpoint_1
                .apply_transaction(
                    make_sign_transaction(
                        &alice,
                        i + 1000,
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
        }
        for i in 0..100 {
            let amount = 100;
            assert!(intermediate
                .apply_transaction(
                    &make_sign_transaction(
                        &alice,
                        i + 1000,
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
        }

        assert_eq!(
            checkpoint_1.account_state(&alice.address),
            intermediate.account_state(&alice.address)
        );
        assert_eq!(checkpoint_1.root_hash().unwrap(), intermediate.root());
    }
}
