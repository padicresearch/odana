use std::collections::{BTreeMap, HashMap};
use std::env::temp_dir;
use std::option::Option::Some;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::RwLockReadGuard;
use std::time::SystemTime;

use anyhow::{Error, Result};
use chrono::Utc;
use merk::{Merk, Op};
use merk::proofs::Query;
use merk::test_utils::TempMerk;
use rand::RngCore;
use rocksdb::checkpoint::Checkpoint;
use rocksdb::ColumnFamily;
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tiny_keccak::{Hasher, Sha3};

use codec::{Codec, Decoder, Encoder};
use codec::impl_codec;
use crypto::SHA256;
use primitive_types::{H160, H256};
use traits::StateDB;
use transaction::{NoncePricedTransaction, TransactionsByNonceAndPrice};
use types::account::{AccountState, get_address_from_pub_key};
use types::Hash;
use types::tx::{Transaction, TransactionKind};

use crate::error::MorphError;
use crate::kv::Schema;
use crate::store::{
    AccountMetadataStorage, AccountStateStorage, column_families, default_db_opts,
    HistorySequenceStorage, HistoryStorage,
};

mod error;
mod kv;
mod snapshot;
mod store;

const GENESIS_ROOT: [u8; 32] = [0; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadProof {
    proof: Vec<u8>,
    root: Hash,
}

#[derive(Clone)]
pub struct Morph {
    db: Arc<RwLock<Merk>>,
}
unsafe impl Sync for Morph {}
unsafe impl Send for Morph {}

impl StateDB for Morph {
    fn nonce(&self, address: &H160) -> u64 {
        let db = self.db.clone();
        let db = match db.read().map_err(|e| anyhow::anyhow!("{}", e)) {
            Ok(db) => db,
            Err(_) => return 0,
        };
        match self
            .get_account_state(address, &db)
            .map(|account_state| account_state.nonce as u64)
        {
            Ok(nonce) => nonce,
            _ => 0,
        }
    }

    fn account_state(&self, address: &H160) -> AccountState {
        let db = self.db.clone();
        let db = match db.read().map_err(|e| anyhow::anyhow!("{}", e)) {
            Ok(db) => db,
            Err(_) => return AccountState::default(),
        };
        match self.get_account_state(address, &db) {
            Ok(state) => state,
            _ => AccountState::default(),
        }
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
        self.apply_operation(action)?;
        Ok(self.root_hash().unwrap())
    }

    fn debit_balance(&self, address: &H160, amount: u128) -> Result<Hash> {
        let action = MorphOperation::DebitBalance {
            account: *address,
            amount,
            tx_hash: [0; 32],
        };
        self.apply_operation(action)?;
        Ok(self.root_hash().unwrap())
    }

    fn snapshot(&self) -> Result<Arc<dyn StateDB>> {
        Ok(Arc::new(self.intermediate()?))
    }

    fn checkpoint(&self, path: String) -> Result<Arc<dyn StateDB>> {
        let state = self.checkpoint(&path)?;
        Ok(Arc::new(state))
    }

    fn apply_txs(&self, txs: Vec<Transaction>) -> Result<Hash> {
        self.apply_txs(txs)?;
        self.root_hash()
    }

    fn root_hash(&self) -> Hash {
        self.root_hash().unwrap()
    }
}

impl Morph {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let merk = merk::Merk::open_opt(path, default_db_opts())?;
        let morph = Self {
            db: Arc::new(RwLock::new(merk)),
        };
        Ok(morph)
    }

    pub fn apply_txs(&self, txs: Vec<Transaction>) -> Result<()> {
        let db = self.db.clone();
        let mut db = db.write().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut accounts: BTreeMap<H160, TransactionsByNonceAndPrice> = BTreeMap::new();
        let mut states: BTreeMap<H160, AccountState> = BTreeMap::new();

        for tx in txs {
            let account = tx.origin();
            let mut list = accounts
                .entry(account)
                .or_insert(TransactionsByNonceAndPrice::default());
            list.insert(NoncePricedTransaction(tx));
        }

        for (acc, _) in accounts.iter() {
            let key = acc.to_fixed_bytes();
            let value = db.get(&key)?;
            let account_state = match value {
                None => AccountState::default(),
                Some(byte) => AccountState::decode(&byte).unwrap_or_default(),
            };
            states.insert(*acc, account_state);
        }

        let mut state_transitions = Vec::new();
        for (_, txs) in accounts {
            for tx in txs {
                self.apply_transaction(tx.0, &mut states)?;
            }
        }
        for (acc, state) in states {
            state_transitions.push((acc, state))
        }
        self.commit(state_transitions, &mut db)
    }

    fn apply_transaction(
        &self,
        transaction: Transaction,
        states: &mut BTreeMap<H160, AccountState>,
    ) -> Result<()> {
        //TODO: verify transaction (probably)
        for action in get_operations(&transaction) {
            let address = action.get_address();
            let account_state = states.get(&address).cloned().unwrap_or_default();
            let new_account_state = self.apply_action(&action, account_state)?;
            states.insert(address, new_account_state);
        }
        Ok(())
    }

    fn apply_operation(&self, action: MorphOperation) -> Result<()> {
        let mut db = self.db.write().map_err(|e| anyhow::anyhow!("{}", e))?;

        let current_account_state = self.get_account_state(&action.get_address(), &db)?;
        let new_account_state = self.apply_action(&action, current_account_state)?;
        let batch = vec![(action.get_address(), new_account_state)];
        self.commit(batch, &mut db)
    }

    fn commit(&self, state_transitions: Vec<(H160, AccountState)>, db: &mut Merk) -> Result<()> {
        let mut batch = Vec::with_capacity(state_transitions.len());
        for (addr, state) in state_transitions {
            let addr = addr.encode()?;
            let state = state.encode()?;
            batch.push((addr, Op::Put(state)))
        }
        db.apply(&batch, &[])?;
        Ok(())
    }

    //fn commit(&self, new_root : )

    pub fn check_transaction(&self, transaction: &Transaction) -> Result<()> {
        Ok(())
    }

    fn apply_action(
        &self,
        action: &MorphOperation,
        account_state: AccountState,
    ) -> Result<AccountState> {
        let mut account_state = account_state;
        match action {
            MorphOperation::DebitBalance { amount, .. } => {
                if account_state.free_balance < *amount {
                    return Err(MorphError::InsufficientFunds.into());
                }
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance { amount, .. } => {
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { nonce, .. } => {
                if *nonce <= account_state.nonce {
                    return Err(MorphError::NonceIsLessThanCurrent.into());
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, address: &H160, db: &Merk) -> Result<AccountState> {
        let key = address.to_fixed_bytes();
        let value = db.get(&key)?;
        match value {
            None => Ok(AccountState::default()),
            Some(byte) => AccountState::decode(&byte),
        }
    }

    fn get_account_state_with_proof(
        &self,
        address: &H160,
    ) -> Result<(Option<AccountState>, ReadProof)> {
        let db = self.db.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut query = Query::new();
        query.insert_key(address.encode()?);
        let account_state = match db.get(&address.encode()?)? {
            None => None,
            Some(value) => Some(AccountState::decode(&value)?),
        };

        let root = db.root_hash();
        let proof = db.prove(query)?;
        Ok((account_state, ReadProof { proof, root }))
    }

    pub fn checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<Self> {
        let db = self.db.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let merk = db.checkpoint(path)?;
        Ok(Self {
            db: Arc::new(RwLock::new(merk)),
        })
    }

    pub fn intermediate(&self) -> Result<Self> {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut path = temp_dir();
        path.push(format!("merk-temp–{}", time));
        let db = self.db.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        let merk = db.checkpoint(path)?;
        Ok(Self {
            db: Arc::new(RwLock::new(merk)),
        })
    }

    pub fn root_hash(&self) -> Result<Hash> {
        let db = self.db.read().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(db.root_hash())
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
                account: H160::from(from),
                amount: *amount + *fee,
                tx_hash,
            });
            ops.push(MorphOperation::CreditBalance {
                account: H160::from(to),
                amount: *amount,
                tx_hash,
            });
            ops.push(MorphOperation::UpdateNonce {
                account: H160::from(from),
                nonce: tx.nonce(),
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
    use tempdir::TempDir;

    use account::create_account;
    use transaction::make_sign_transaction;

    use super::*;

    #[test]
    fn test_morph() {
        let path = TempDir::new("state").unwrap();
        let mut morph = Morph::new(path.path()).unwrap();
        let alice = create_account();
        let bob = create_account();
        let jake = create_account();
        morph.credit_balance(&alice.address, 1_000_000).unwrap();
        let mut txs = Vec::new();
        for i in 0..100 {
            let amount = 100;
            let tx = make_sign_transaction(
                &alice,
                i + 1,
                TransactionKind::Transfer {
                    from: alice.address.to_fixed_bytes(),
                    to: bob.address.to_fixed_bytes(),
                    amount,
                    fee: (amount as f64 * 0.01) as u128,
                },
            )
            .unwrap();
            txs.push(tx);
        }
        morph.apply_txs(txs).unwrap();

        println!("Alice: {:#?}", morph.account_state(&alice.address));
        println!("Bob: {:#?}", morph.account_state(&bob.address));
        let s2 = path.into_path().join("s2");
        let checkpoint_1 = morph.checkpoint(s2.as_path()).unwrap();
        let mut intermediate = morph.intermediate().unwrap();
        for i in 0..100 {
            let amount = 100;
            assert!(checkpoint_1
                .apply_txs(vec![make_sign_transaction(
                    &alice,
                    i + 1000,
                    TransactionKind::Transfer {
                        from: alice.address.to_fixed_bytes(),
                        to: bob.address.to_fixed_bytes(),
                        amount,
                        fee: (amount as f64 * 0.01) as u128,
                    },
                )
                .unwrap(),])
                .is_ok());
        }
        for i in 0..100 {
            let amount = 100;
            assert!(intermediate
                .apply_txs(vec![make_sign_transaction(
                    &alice,
                    i + 1000,
                    TransactionKind::Transfer {
                        from: alice.address.to_fixed_bytes(),
                        to: bob.address.to_fixed_bytes(),
                        amount,
                        fee: (amount as f64 * 0.01) as u128,
                    },
                )
                .unwrap()])
                .is_ok());
        }

        assert_eq!(
            checkpoint_1.account_state(&alice.address),
            intermediate.account_state(&alice.address)
        );
        assert_eq!(
            checkpoint_1.root_hash().unwrap(),
            intermediate.root_hash().unwrap()
        );
    }
}
