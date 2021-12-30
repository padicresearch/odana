mod error;

use std::collections::HashMap;
use tiny_keccak::Hasher;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::error::Error;
use storage::codec::{Codec, Encoder};
use storage::{KVStore, KVEntry};
use storage::impl_codec;
use types::TxHash;
use account::Account;
use chrono::Utc;


type AccountId = [u8; 32];
type Hash = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProofElement {
    Operation { op: MorphOperation, seq: usize },
    StateHash { seq: usize },
}

pub type MorphStorageKV<C> = dyn KVStore<Morph<C>> + Send + Sync;

pub trait Config: SudoAccount + TreasuryAccount {}

#[derive(Clone, Debug)]
pub struct Morph<Config> {
    kv: MorphStorageKV<Config>,
    log: Vec<MorphOperation>,
    history: Vec<Hash>,
    config: Config,
}


impl<C> KVEntry for Morph<C> {
    type Key = AccountId;
    type Value = AccountState;

    fn column() -> &'static str {
        "balances"
    }
}

impl<C> Morph<C> where C : Config {
    pub fn new(genesis_balances: HashMap<AccountId, AccountState>) -> Self {
        Self {
            kv: genesis_balances,
            log: vec![],
            history: vec![[0_u8; 32]],
            config: Config,
        }
    }
    pub fn add_transaction(&mut self, transaction: MorphTransaction) -> Result<()> {
        verify_signed_transaction(&transaction)?;
        for action in transaction.get_operations(&self.config).iter() {
            let new_account_state = self.apply_action(action)?;
            let mut sha3 = tiny_keccak::Sha3::v256();
            sha3.update(self.history.last().ok_or(Error::TransactionFailed)?);
            sha3.update(&bincode::serialize(action)?);
            sha3.update(&bincode::serialize(&new_account_state)?);
            let mut new_root = [0; 32];
            sha3.finalize(&mut new_root);
            self.history.push(new_root);
            self.log.push(action.clone());
        }
        Ok(())
    }

    pub fn check_transaction(&mut self, transaction: &MorphTransaction) -> Result<()> {
        Ok(())
    }

    fn apply_action(&mut self, action: &MorphOperation) -> Result<AccountState> {
        match action {
            MorphOperation::Debit { account, amount, .. } => {
                let mut account_state = self.kv.get(account)?.or_else(Error::AccountNotFound.into())?;
                account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                self.kv.put(*account, account_state);
                Ok(account_state)
            }
            MorphOperation::Credit { account, amount, .. } => {
                let mut account_state = self.kv.get(account)?.unwrap_or_default();
                account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                self.kv.put(*account, account_state);
                Ok(account_state)
            }
        }
    }

    fn get_account_state(&self, account_id: &Hash) -> Result<Option<AccountState>> {
        self.kv.get(account_id)
    }

    fn proof(&self, account_id: u128) -> Vec<ProofElement> {
        let mut proof = Vec::new();
        for (idx, op) in self.log.iter().enumerate() {
            let account = match op {
                MorphOperation::Debit { account, .. } => {
                    account
                }
                MorphOperation::Credit { account, .. } => {
                    account
                }
            };

            if *account == account_id {
                proof.push(ProofElement::Operation { op: op.clone(), seq: idx + 1 })
            } else {
                proof.push(ProofElement::StateHash { seq: idx + 1 });
            }
        }
        proof
    }

    fn compact_proof(&self, account_id: u128) -> Result<Vec<ProofElement>> {
        let mut proof = Vec::new();
        for (idx, op) in self.log.iter().enumerate() {
            let account = match op {
                MorphOperation::Debit { account, .. } => {
                    account
                }
                MorphOperation::Credit { account, .. } => {
                    account
                }
            };

            if *account == account_id {
                proof.push(ProofElement::Operation { op: op.clone(), seq: idx + 1 })
            } else {
                if let Some(ProofElement::StateHash { .. }) = proof.last() {
                    let d = proof.len() - 1;
                    std::mem::swap(proof.get_mut(d).ok_or(anyhow!("poof index not found"))?, &mut ProofElement::StateHash { seq: idx + 1 })
                    //proof.push();
                } else {
                    proof.push(ProofElement::StateHash { seq: idx + 1 });
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

pub fn validate_account_state<C>(proof: &Vec<ProofElement>, morph: &Morph<C>) -> Result<(AccountState, usize), Error> where C : Config {
    let mut account_state: AccountState = AccountState::default();
    let mut history = vec![[0; 32]];
    let mut last_valid_seq = 0;
    for el in proof {
        match el {
            ProofElement::Operation { op, seq } => {
                match op {
                    MorphOperation::Debit { amount, .. } => {
                        account_state.free_balance = account_state.free_balance.saturating_sub(*amount);
                    }
                    MorphOperation::Credit { amount, .. } => {
                        account_state.free_balance = account_state.free_balance.saturating_add(*amount);
                    }
                }

                let account_state = account_state.ok_or(())?;
                let mut sha3 = tiny_keccak::Sha3::v256();
                sha3.update(history.last().unwrap());
                sha3.update(&bincode::serialize(op)?);
                sha3.update(&bincode::serialize(&account_state)?);
                let mut new_root = [0; 32];
                sha3.finalize(&mut new_root);


                let valid_history_hash = morph.history.get(*seq).ok_or((Error::ValidationFailedHistoryNotFound)?;
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

    let account_state = account_state.ok_or(())?;
    Ok((account_state, last_valid_seq))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MorphOperation {
    Debit { account: AccountId, amount: u128, tx_hash: Hash },
    Credit { account: AccountId, amount: u128, tx_hash: Hash },
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountState {
    free_balance: u128,
    reserve_balance: u128,
    nonce : u32
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            free_balance: 0,
            reserve_balance: 0,
            nonce: 0
        }
    }
}

pub type Sig = [u8; 64];

#[derive(Serialize, Deserialize, Clone)]
pub enum MorphTransactionKind {
    Transfer { from: AccountId, to: AccountId, amount: u128, nonce: u32 },
    Coinbase { miner: AccountId, amount: u128 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MorphTransaction {
    sig: Sig,
    origin: AccountId,
    nonce: u32,
    #[serde(flatten)]
    kind: MorphTransactionKind,
}

impl MorphTransaction {
    fn new(origin: AccountId, nonce: u32, sig: Sig, kind: MorphTransactionKind) -> Result<Self> {
        Ok(Self {
            sig,
            origin,
            nonce,
            kind,
        })
    }

    pub fn origin(&self) -> &AccountId {
        &self.origin
    }

    pub fn id(&self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        match self.encode() {
            Ok(encoded_self) => {
                sha3.update(&encoded_self);
            }
            Err(_) => {}
        }
        sha3.finalize(&mut out);
        out
    }

    pub fn signature(&self) -> &Sig {
        &self.sig
    }
    pub fn nonce(&self) -> [u8; 4] {
        self.nonce.to_be_bytes()
    }

    pub fn get_operations<Config>(&self, config : &Config) -> Vec<MorphOperation> where Config : TreasuryAccount {
        let mut ops = Vec::new();
        let tx_hash = self.id();
        match &self.kind {
            MorphTransactionKind::Transfer { from, to, amount, .. } => {
                ops.push(MorphOperation::Debit { account: *from, amount: *amount, tx_hash });
                ops.push(MorphOperation::Credit { account: *to, amount: *amount, tx_hash });
            }
            MorphTransactionKind::Coinbase{amount, miner} => {
                ops.push(MorphOperation::Debit { account: config.treasury(), amount: *amount, tx_hash });
                ops.push(MorphOperation::Credit { account: *miner, amount: *amount, tx_hash });
            }
        }
        ops
    }

    pub fn sig_hash(&self) -> Box<[u8]> {
        let mut out = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(self.origin());
        sha3.update(&self.nonce());
        sha3.update(&self.kind.encode()?);
        sha3.finalize(&mut out);
        Box::new(*out)
    }
}

pub fn make_sign_transaction(account: &Account, nonce: u32, kind: MorphTransactionKind) -> Result<MorphTransaction> {
    let mut out = [0_u8; 32];
    let mut sha3 = tiny_keccak::Sha3::v256();
    sha3.update(&account.pub_key);
    sha3.update(&nonce.to_be_bytes());
    sha3.update(&kind.encode()?);
    sha3.finalize(&mut out);

    let sig = account.sign(&out)?;
    Ok(MorphTransaction::new(*account.pub_key, nonce, sig, kind))
}

pub fn verify_signed_transaction(transaction: &MorphTransaction) -> Result<()> {
    account::verify_signature(transaction.origin(), transaction.signature(), &transaction.sig_hash())
}

pub fn verify_transaction_origin(origin: &[u8; 32], transaction: &MorphTransaction) -> Result<()> {
    account::verify_signature(origin, transaction.signature(), &transaction.sig_hash())
}

impl_codec!(MorphTransaction);
impl_codec!(MorphTransactionKind);
impl_codec!(AccountState);

#[cfg(test)]
mod tests {
    use crate::{Morph, AccountState, MorphOperation, validate_account_state};
    use std::collections::HashMap;
    use crate::MorphOperation::{Debit, Credit};

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