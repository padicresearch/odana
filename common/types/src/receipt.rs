use primitive_types::H256;

use primitive_types::address::Address;
use serde::{Deserialize, Serialize};

pub type Log = Vec<u8>;

#[derive(Serialize, Deserialize, Clone, prost::Message)]
pub struct Receipt {
    #[prost(required, message, tag = "1")]
    app_id: Address,
    #[prost(required, message, tag = "2")]
    tx_hash: H256,
    #[prost(repeated, bytes, tag = "3")]
    logs: Vec<Log>,
    #[prost(uint64, tag = "4")]
    #[serde(with = "hex")]
    fuel_used: u64,
    #[prost(required, message, tag = "5")]
    post_state: H256,
    #[prost(bool, tag = "6")]
    status: bool,
}

impl Receipt {
    pub fn app_id(&self) -> Address {
        self.app_id
    }
    pub fn tx_hash(&self) -> H256 {
        self.tx_hash
    }
    pub fn logs(&self) -> &Vec<Log> {
        &self.logs
    }
    pub fn fuel_used(&self) -> u64 {
        self.fuel_used
    }

    // pub fn new(
    //     app_id: u32,
    //     tx_hash: H256,
    //     logs: Vec<Log>,
    //     fuel_used: u64,
    //     post_state: H256,
    //     status: bool,
    // ) -> Self {
    //     Self {
    //         app_id,
    //         tx_hash,
    //         logs,
    //         fuel_used,
    //         post_state: post_state,
    //         status,
    //     }
    // }
    // pub fn post_state(&self) -> H256 {
    //     self.post_state
    // }
    // pub fn status(&self) -> bool {
    //     self.status
    // }
}
