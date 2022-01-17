use std::time::Duration;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

use codec::{Decoder, Encoder};
use codec::impl_codec;

pub mod account;
pub mod block;
pub mod events;
pub mod tx;

pub type Hash = [u8; 32];
pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MerkleHash = [u8; 32];
pub type Sig = [u8; 64];
pub type PubKey = [u8; 32];

#[derive(Serialize, Deserialize, Getters, Debug, Clone)]
pub struct MempoolSnapsot {
    pub pending: Vec<TxHash>,
    pub valid: Vec<TxHash>,
}

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

impl_codec!(MempoolSnapsot);

big_array! { BigArray; }
