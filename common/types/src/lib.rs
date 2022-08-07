#![feature(test)]
extern crate test;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

use codec::impl_codec;
use codec::{Decoder, Encoder};
use primitive_types::H160;

use crate::block::BlockHeader;

pub mod account;
pub mod block;
pub mod config;
pub mod events;
pub mod network;
pub mod tx;
mod uint_hex_codec;

pub type Hash = [u8; 32];
pub type Address = [u8; 20];

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

pub fn cache<F, T>(hash: &Arc<RwLock<Option<T>>>, f: F) -> T
where
    F: Fn() -> T,
    T: Copy + Clone,
{
    if let Ok(hash) = hash.read() {
        if let Some(hash) = *hash {
            return hash;
        }
    }
    let out = f();
    if let Ok(mut hash) = hash.write() {
        *hash = Some(out)
    }
    out
}

big_array! { BigArray; +33,65}
