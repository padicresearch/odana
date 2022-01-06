use traits::StateDB;
use std::sync::Arc;
use dashmap::DashMap;
use primitive_types::H160;
use types::AccountId;
use std::collections::hash_map::RandomState;
use dashmap::mapref::one::Ref;

#[derive(Clone)]
pub struct TxNoncer<State> {
    fallback: State,
    nonces: Arc<DashMap<AccountId, u64>>,
}

impl<State> TxNoncer<State> where State: StateDB {
    pub fn new(state: State) -> Self {
        Self {
            fallback: state,
            nonces: Arc::new(Default::default()),
        }
    }
    pub fn get(&self, account_id: &AccountId) -> u64 {
        *self.nonces.entry(*account_id).or_insert_with(|| {
            self.fallback.account_nonce(account_id)
        }).value()
    }

    pub fn set(&self, account_id: AccountId, nonce: u64) {
        self.nonces.insert(account_id, nonce);
    }

    pub fn set_if_lower(&self, account_id: AccountId, nonce: u64) {
        let mut entry = self.nonces.entry(account_id).or_insert_with(|| {
            self.fallback.account_nonce(&account_id)
        });

        if *entry.value() <= nonce {
            return
        }

        *entry.value_mut() = nonce;
    }

    pub fn set_all(&mut self, all: Box<dyn Iterator<Item=(AccountId, u64)>>) {
        self.nonces.clear();
        for (k, v) in all {
            self.nonces.insert(k, v);
        }
    }
}

