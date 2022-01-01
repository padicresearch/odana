mod error;
mod snapshot;
mod logdb;

use std::collections::HashMap;
use tiny_keccak::Hasher;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::error::Error;
use codec::{Codec, Encoder, Decoder};
use storage::{KVStore, KVEntry};
use codec::impl_codec;
use types::{TxHash, BlockHash, AccountId};
use account::{Account, TREASURY_ACCOUNT_PK};
use chrono::Utc;
use transaction::{Transaction, TransactionKind, verify_signed_transaction};
use std::sync::Arc;
use crate::snapshot::MorphSnapshot;

type Hash = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProofElement {
    Operation { op: MorphOperation, seq: usize },
    StateHash { seq: usize },
}

pub type MorphStorageKV = dyn KVStore<Morph> + Send + Sync;


#[derive(Clone)]
pub struct Morph {
    kv: Arc<MorphStorageKV>,
    log: Vec<MorphOperation>,
    history: Vec<Hash>,
}

impl KVEntry for Morph {
    type Key = AccountId;
    type Value = AccountState;

    fn column() -> &'static str {
        "account_balances"
    }
}

impl Morph {
    pub fn new(kv: Arc<MorphStorageKV>) -> Result<Self> {
        let mut morph = Self {
            kv,
            log: vec![],
            history: vec![],
        };
        Ok(morph)
    }

    pub fn apply_transaction(&mut self, transaction: Transaction) -> Result<()> {
        //TODO: verify transaction (probably)
        for action in get_operations(&transaction).iter() {
            let mut new_account_state = self.apply_action(action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            let current_root = match self.history.last() {
                None => {
                    if self.history.len() > 0 {
                        return Err(Error::ValidationFailedRootNotValid.into())
                    }
                    [0_u8;32]
                }
                Some(root) => {
                    *root
                }
            };
            sha3.update(&current_root);
            sha3.update(&action.encode()?);
            sha3.update(&new_account_state.encode()?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);
            self.history.push(new_root);
            self.log.push(action.clone());
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
            MorphOperation::DebitBalance { account, amount, .. } => {
                let mut account_state = self.kv.get(account)?.ok_or(Error::AccountNotFound)?;
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                Ok(account_state)
            }
            MorphOperation::CreditBalance { account, amount, .. } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                Ok(account_state)
            }
            MorphOperation::UpdateNonce { account, nonce,.. } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                if *nonce < account_state.nonce {
                    return Err(Error::NonceIsLessThanCurrent.into())
                }
                account_state.nonce = *nonce;
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, account_id: &Hash) -> Result<Option<AccountState>> {
        self.kv.get(account_id)
    }

    fn proof(&self, account_id: AccountId) -> Vec<ProofElement> {
        let mut proof = Vec::new();
        for (idx, op) in self.log.iter().enumerate() {
            let account =op.get_account_id();
            if account == account_id {
                proof.push(ProofElement::Operation { op: op.clone(), seq: idx })
            } else {
                proof.push(ProofElement::StateHash { seq: idx });
            }
        }
        proof
    }

    pub fn create_snapshot(&self) -> Result<MorphSnapshot> {
        MorphSnapshot::new(self)
    }

    pub fn commit_snapshot(&self, snapshot: MorphSnapshot) -> Result<()> {
        Ok(())
    }

    fn compact_proof(&self, account_id: AccountId) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for (idx, op) in self.log.iter().enumerate() {
            let account =op.get_account_id();

            if account == account_id {
                proof.push(ProofElement::Operation { op: op.clone(), seq: idx })
            } else {
                if let Some(ProofElement::StateHash { .. }) = proof.last() {
                    let d = proof.len() - 1;
                    std::mem::swap(proof.get_mut(d).ok_or(anyhow::anyhow!("poof index not found"))?, &mut ProofElement::StateHash { seq: idx })
                    //proof.push();
                } else {
                    proof.push(ProofElement::StateHash { seq: idx });
                }
            }
        }
        Ok(proof)
    }

    pub fn root_hash(&self) -> &Hash {
        self.history.last().unwrap()
    }
}

// fn hash_op(prev_hash : &Hash, op : &MorphOperation) -> Hash {
//
// }

pub fn validate_account_state(proof: &Vec<ProofElement>, morph: &Morph) -> Result<(AccountState, usize), Error>{
    let mut account_state: AccountState = AccountState::default();
    let mut history : Vec<[u8;32]> = vec![];
    let mut last_valid_seq = 0;
    for el in proof {
        match el {
            ProofElement::Operation { op, seq } => {
                match op {
                    MorphOperation::DebitBalance { amount, .. } => {
                        account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                    }
                    MorphOperation::CreditBalance { amount, .. } => {
                        account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                    }
                    MorphOperation::UpdateNonce { nonce, .. } => {
                        account_state.nonce = *nonce;
                    }
                }
                let prev_root = history.last().unwrap_or(&[0;32]);
                let mut sha3 = tiny_keccak::Sha3::v256();
                sha3.update(prev_root);
                sha3.update(&op.encode()?);
                sha3.update(&account_state.encode()?);
                let mut new_root = [0; 32];
                sha3.finalize(&mut new_root);


                let valid_history_hash = morph.history.get(*seq).ok_or((Error::ValidationFailedHistoryNotFound))?;

                println!("valid_history_hash: {}", hex::encode(&valid_history_hash));
                println!("calcu_history_hash: {}", hex::encode(&new_root));
                if &new_root != valid_history_hash {
                    return Err(Error::ValidationFailedRootNotValid);
                }
                history.push(new_root);
                last_valid_seq = *seq
            }
            ProofElement::StateHash { seq } => {
                history.push(*morph.history.get(*seq).ok_or(Error::ValidationFailedHistoryNotFound)?);
            }
        }
    }

    Ok((account_state, last_valid_seq))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MorphOperation {
    DebitBalance { account: AccountId, amount: u128, tx_hash: Hash },
    CreditBalance { account: AccountId, amount: u128, tx_hash: Hash },
    UpdateNonce { account: AccountId, nonce: u32, tx_hash: Hash }
}

impl MorphOperation {
    fn get_account_id(&self) -> AccountId {
        match self {
            MorphOperation::DebitBalance { account, .. } => {
                *account
            }
            MorphOperation::CreditBalance { account, .. } => {
                *account
            }
            MorphOperation::UpdateNonce { account,.. } => {
                *account
            }
        }
    }
}

pub fn get_operations(tx: &Transaction) -> Vec<MorphOperation> {
    let mut ops = Vec::new();
    let tx_hash = tx.id();
    match tx.kind() {
        TransactionKind::Transfer { from, to, amount, .. } => {
            ops.push(MorphOperation::DebitBalance { account: *from, amount: *amount, tx_hash });
            ops.push(MorphOperation::CreditBalance { account: *to, amount: *amount, tx_hash });
            ops.push(MorphOperation::UpdateNonce { account: *from, nonce: tx.nonce_u32(), tx_hash });
        }
        TransactionKind::Coinbase { amount, miner, .. } => {
            ops.push(MorphOperation::CreditBalance { account: *miner, amount: *amount, tx_hash });
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
    use storage::memstore::MemStore;
    use account::create_account;
    use transaction::make_sign_transaction;

    #[test]
    fn test_morph() {
        let memstore = Arc::new(MemStore::new(vec![Morph::column()]));
        let mut morph = Morph::new(memstore).unwrap();
        let alice = create_account();
        let bob = create_account();
        let jake = create_account();

        morph.apply_transaction(make_sign_transaction(&alice, 1, TransactionKind::Coinbase {
            miner: alice.pub_key,
            block_hash: bob.pub_key,
            amount: 10000000,
        }).unwrap());

        morph.apply_transaction(make_sign_transaction(&alice, 1, TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 10,
        }).unwrap());

        println!("Alice {:#?}", morph.get_account_state(&alice.pub_key).unwrap().unwrap());
        println!("Bob {:#?}", morph.get_account_state(&bob.pub_key).unwrap().unwrap());
        println!("----------------------------------------------------------------------------------------------------------------");
        println!("Alice Proof: {:?}", morph.compact_proof(alice.pub_key));
        println!("Bob Proof: {:?}", morph.compact_proof(bob.pub_key));
       // validate_account_state(&morph.compact_proof(alice.pub_key).unwrap(), &morph).unwrap();
        validate_account_state(&morph.compact_proof(bob.pub_key).unwrap(), &morph).unwrap();
        //assert!()
    }

    // #[test]
    // fn test_morph() {
    //     let mut genesis = HashMap::new();
    //     genesis.insert(0, AccountState {
    //         free_balance: 1000000000,
    //         reserve_balance: 0
    //     });
    //     let mut morph = Morph::new(genesis);
    //     morph.dispatch(&vec![MorphOperation::RegisterAccount(1)]);
    //     morph.dispatch(&vec![MorphOperation::RegisterAccount(2)]);
    //     morph.dispatch(&vec![MorphOperation::RegisterAccount(3)]);
    //
    //     println!("{:?}", morph.root_hash());
    //     println!("{:?}", morph);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 1, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 2, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 2, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 1 }, Credit { account: 1, amount: 1 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 2, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 1, amount: 20000 }, Credit { account: 2, amount: 20000 }]);
    //     println!("{:?}", morph.root_hash());
    //     println!("{:?}", morph);
    //     println!("Compact Proof {:?}", morph.compact_proof(1).len());
    //     println!("Compact Proof {:?}", morph.compact_proof(1));
    //     println!("Proof {:?}", morph.proof(1).len());
    //     println!("Proof {:?}", morph.proof(1));
    //     println!("History {:?}", morph.history);
    //     let (account_state,at) = validate_account_state(&morph.proof(1), &morph).unwrap();
    //     let (c_account_state,at) = validate_account_state(&morph.compact_proof(1), &morph).unwrap();
    //     assert_eq!(account_state, morph.get_account_state(&1).unwrap());
    //     assert_eq!(c_account_state, morph.get_account_state(&1).unwrap());
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 1, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 1, amount: 3200000 }]);
    //     morph.dispatch(&vec![Debit { account: 0, amount: 3200000 }, Credit { account: 1, amount: 3200000 }]);
    // }
}