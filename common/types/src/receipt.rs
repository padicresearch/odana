use primitive_types::H256;

use serde::{Deserialize, Serialize};

pub type Log = Vec<u8>;

#[derive(Serialize, Deserialize, Clone, prost::Message)]
pub struct Receipt {
    #[prost(uint32, tag = "1")]
    app_id: u32,
    #[prost(bytes, tag = "2")]
    tx_hash: Vec<u8>,
    #[prost(repeated, bytes, tag = "3")]
    logs: Vec<Log>,
    #[prost(uint64, tag = "4")]
    fuel_used: u64,
}

impl Receipt {
    pub fn app_id(&self) -> u32 {
        self.app_id
    }
    pub fn tx_hash(&self) -> H256 {
        H256::from_slice(&self.tx_hash)
    }
    pub fn logs(&self) -> &Vec<Log> {
        &self.logs
    }
    pub fn fuel_used(&self) -> u64 {
        self.fuel_used
    }

    pub fn new(app_id: u32, tx_hash: Vec<u8>, logs: Vec<Log>, fuel_used: u64) -> Self {
        Self {
            app_id,
            tx_hash,
            logs,
            fuel_used,
        }
    }
}
