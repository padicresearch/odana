#![feature(test)]
#![feature(slice_take)]
extern crate core;
extern crate test;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::account::AccountState;
use bytes::{Buf, BufMut};
use codec::{Decodable, Encodable};
use parking_lot::RwLock;
use primitive_types::address::Address;
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use serde::{Deserialize, Serialize};
use smt::SparseMerkleTree;

use crate::block::BlockHeader;
use crate::network::Network;

pub mod account;
pub mod app;
pub mod block;
pub mod config;
pub mod events;
pub mod misc;
pub mod network;
pub mod receipt;
pub mod tx;
pub mod util;

pub type Hash = [u8; 32];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChainStateValue {
    CurrentHeader(BlockHeader),
}

impl Default for ChainStateValue {
    fn default() -> Self {
        Self::CurrentHeader(BlockHeader::default())
    }
}

impl prost::Message for ChainStateValue {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        match self {
            ChainStateValue::CurrentHeader(header) => {
                prost::encoding::message::encode(1, header, buf)
            }
        }
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        match tag {
            1 => match self {
                ChainStateValue::CurrentHeader(header) => {
                    prost::encoding::message::merge(wire_type, header, buf, ctx)
                }
            },
            _ => panic!("invalid ChainStateValue tag: {}", tag),
        }
    }

    fn encoded_len(&self) -> usize {
        match self {
            ChainStateValue::CurrentHeader(header) => {
                prost::encoding::message::encoded_len(1u32, header)
            }
        }
    }

    fn clear(&mut self) {}
}

impl Encodable for ChainStateValue {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.encode_to_vec())
    }
}

impl Decodable for ChainStateValue {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        <Self as prost::Message>::decode(buf).map_err(|e| e.into())
    }
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

pub fn cache<F, T>(hash: &Arc<RwLock<Option<T>>>, f: F) -> T
where
    F: Fn() -> anyhow::Result<T>,
    T: Copy + Clone + Default,
{
    {
        let hash = hash.read();
        if let Some(hash) = *hash {
            return hash;
        }
    }
    let mut hash = hash.write();
    match f() {
        Ok(out) => {
            *hash = Some(out);
            out
        }
        Err(_) => T::default(),
    }
}

pub trait Addressing {
    fn is_mainnet(&self) -> bool;
    fn is_testnet(&self) -> bool;
    fn is_alphanet(&self) -> bool;
    fn is_valid(&self) -> bool;
    fn network(&self) -> Option<Network>;
}
pub struct Changelist {
    pub account_changes: HashMap<Address, AccountState>,
    pub logs: Vec<(String, Vec<u8>)>,
    pub storage: SparseMerkleTree,
}

pub mod prelude {
    pub use crate::account::*;
    pub use crate::block::*;
    pub use crate::config::*;
    pub use crate::events::*;
    pub use crate::network::*;
    pub use crate::tx::*;
    pub use crate::Addressing;
}
