use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use primitive_types::Address;

use traits::StateDB;


#[derive(Clone)]
pub struct TxNoncer {
    fallback: Arc<dyn StateDB>,
    nonces: Arc<DashMap<Address, u64>>,
}

impl TxNoncer {
    pub fn new(state: Arc<dyn StateDB>) -> Self {
        Self {
            fallback: state,
            nonces: Arc::new(Default::default()),
        }
    }
    pub fn get(&self, address: &Address) -> u64 {
        *self
            .nonces
            .entry(*address)
            .or_insert_with(|| self.fallback.nonce(address))
            .value()
    }

    pub fn set(&self, account_id: Address, nonce: u64) {
        self.nonces.insert(account_id, nonce);
    }

    pub fn set_if_lower(&self, address: Address, nonce: u64) {
        let mut entry = self
            .nonces
            .entry(address)
            .or_insert_with(|| self.fallback.nonce(&address));

        if *entry.value() <= nonce {
            return;
        }

        *entry.value_mut() = nonce;
    }

    pub fn set_all(&mut self, all: &HashMap<Address, u64>) {
        self.nonces.clear();
        for (k, v) in all {
            self.nonces.insert(*k, *v);
        }
    }
}
