use std::sync::{Arc, RwLock};
use std::time::Duration;

use derive_getters::Getters;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

use codec::impl_codec;
use codec::{Decoder, Encoder};
use primitive_types::{H160, H256};

use crate::account::AccountState;
use crate::block::BlockHeader;

pub mod account;
pub mod block;
pub mod config;
pub mod events;
pub mod network;
pub mod tx;

pub type Hash = [u8; 32];
pub type Address = [u8; 20];

#[derive(Serialize, Deserialize, Getters, Debug, Clone)]
pub struct MempoolSnapsot {
    pub pending: Vec<Hash>,
    pub valid: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChainStateValue {
    CurrentHeader(BlockHeader),
}

impl_codec!(ChainStateValue);

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct TxPoolConfig {
    // Whether local transaction handling should be disabled
    pub no_locals: bool,
    // Minimum fee per price of transaction
    pub price_ratio: f64,
    // Minimum price bump percentage to replace an already existing transaction (nonce)
    pub price_bump: u128,
    // Number of executable transaction slots guaranteed per account
    pub account_slots: u64,
    // Maximum number of executable transaction slots for all accounts
    pub global_slots: u64,
    // Maximum number of non-executable transaction slots permitted per account
    pub account_queue: u64,
    // Maximum number of non-executable transaction slots for all accounts
    pub global_queue: u64,
    // Maximum amount of time non-executable transaction are queued
    pub life_time: Duration,
}

pub fn cache_hash<F>(hash: &Arc<RwLock<Option<Hash>>>, f: F) -> Hash
where
    F: Fn() -> Hash,
{
    match hash.read() {
        Ok(hash) => match *hash {
            Some(hash) => return hash,
            None => {}
        },
        Err(_) => {}
    }
    let out = f();
    match hash.write() {
        Ok(mut hash) => *hash = Some(out),
        Err(_) => {}
    }
    out
}

pub struct Genesis {
    chain_id: u32,
    accounts: Vec<(H160, AccountState)>,
    block_header: BlockHeader,
}

impl_codec!(MempoolSnapsot);

big_array! { BigArray; +33,65}
