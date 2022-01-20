use crate::error::Error;
use anyhow::Result;
use primitive_types::{endian, Compact, H160, U256};
use std::sync::Arc;
use traits::{ChainHeadReader, Consensus, StateDB};
use types::account::AccountState;
use types::block::{Block, BlockHeader};
use types::tx::Transaction;
use types::{Genesis, Hash};

pub const MAX_BLOCK_HEIGHT: u128 = 25_000_000;
pub const INITIAL_REWARD: u128 = 10 * 1_000_000_000 /*TODO: Use TUC constant*/;
pub const SPREAD: u128 = MAX_BLOCK_HEIGHT.pow(4) / INITIAL_REWARD;
pub const PRECISION_CORRECTION: u128 = 5012475762;
pub const MAX_SUPPLY_APPROX: u128 =
    (INITIAL_REWARD * MAX_BLOCK_HEIGHT) - (MAX_BLOCK_HEIGHT.pow(5) / (5 * SPREAD));
pub const MAX_SUPPLY_PRECOMPUTED: u128 = MAX_SUPPLY_APPROX + PRECISION_CORRECTION;

#[inline]
pub fn miner_reward(block_height: u128) -> u128 {
    INITIAL_REWARD - block_height.pow(4) / SPREAD
}

mod barossa;
pub mod coin;
mod error;

#[cfg(test)]
mod tests {
    use crate::barossa::{BarossaProtocol, Network};
    use primitive_types::{H256, U256, Compact};
    use traits::Consensus;

    #[test]
    fn print_target() {
        let target_u256 = U256([
            0x0000000000000000u64,
            0x0000000000000000u64,
            0x0000000000000000u64,
            0x00000377ae000000u64,
        ]);
        //let MAX_BITS_MAINNET: U256 = "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".parse().unwrap();
        let mut out = [0; 32];
        target_u256.to_big_endian(&mut out);
        let protocol = BarossaProtocol::new(Network::Testnet, 503543726);
        let mut h = [0; 32];

        U256::from(protocol.max_difficulty().clone()).to_big_endian(&mut h);
        println!("{:?}", hex::encode(h))
    }
}
