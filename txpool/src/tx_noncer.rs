use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use primitive_types::H160;
use std::collections::hash_map::RandomState;
use std::sync::Arc;
use traits::StateDB;
use types::PubKey;

#[derive(Clone)]
pub struct TxNoncer<State> {
    fallback: State,
    nonces: Arc<DashMap<H160, u64>>,
}

impl<State> TxNoncer<State>
where
    State: StateDB,
{
    pub fn new(state: State) -> Self {
        Self {
            fallback: state,
            nonces: Arc::new(Default::default()),
        }
    }
    pub fn get(&self, address: &H160) -> u64 {
        *self
            .nonces
            .entry(*address)
            .or_insert_with(|| self.fallback.account_nonce(address))
            .value()
    }

    pub fn set(&self, account_id: H160, nonce: u64) {
        self.nonces.insert(account_id, nonce);
    }

    pub fn set_if_lower(&self, address: H160, nonce: u64) {
        let mut entry = self
            .nonces
            .entry(address)
            .or_insert_with(|| self.fallback.account_nonce(&address));

        if *entry.value() <= nonce {
            return;
        }

        *entry.value_mut() = nonce;
    }

    pub fn set_all(&mut self, all: Box<dyn Iterator<Item=(H160, u64)>>) {
        self.nonces.clear();
        for (k, v) in all {
            self.nonces.insert(k, v);
        }
    }
}
