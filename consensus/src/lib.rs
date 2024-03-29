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

pub mod barossa;
pub mod coin;
pub mod constants;
pub mod error;
