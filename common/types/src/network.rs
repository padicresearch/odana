use clap::ArgEnum;
use serde::{Deserialize, Serialize};

use primitive_types::{Compact, U256};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, ArgEnum)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Alphanet,
}

impl From<Network> for String {
    fn from(network: Network) -> Self {
        match network {
            Network::Testnet => "testnet".to_string(),
            Network::Alphanet => "aplhanet".to_string(),
            Network::Mainnet => "mainnet".to_string(),
        }
    }
}

const TESTNET_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x00000377ae000000u64,
]);
const ALPHA_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x000000fff000000u64,
]);

pub const CHAIN_PREFIX: &str = "uch";

const MAINNET_MAX_DIFFICULTY: U256 = U256([
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0x00000000ffffffffu64,
]);

pub const TESTNET_HRP: &str = "tuc";
pub const ALPHA_HRP: &str = "luc";
pub const MAINNET_HRP: &str = "uch";

impl Network {
    pub fn max_difficulty(&self) -> U256 {
        match self {
            Network::Testnet => TESTNET_MAX_DIFFICULTY,
            Network::Alphanet => ALPHA_MAX_DIFFICULTY,
            Network::Mainnet => MAINNET_MAX_DIFFICULTY,
        }
    }

    pub fn chain_id(&self) -> u32 {
        match self {
            Network::Mainnet => 0,
            Network::Testnet => 1,
            Network::Alphanet => 2,
        }
    }

    pub fn from_chain_id(chain_id: u32) -> Self {
        match chain_id {
            0 => Network::Mainnet,
            1 => Network::Testnet,
            2 => Network::Alphanet,
            _ => Network::Testnet,
        }
    }
    pub fn hrp(&self) -> &'static str {
        match self {
            Network::Mainnet => MAINNET_HRP,
            Network::Testnet => TESTNET_HRP,
            Network::Alphanet => ALPHA_HRP,
        }
    }

    pub fn max_difficulty_compact(&self) -> Compact {
        match self {
            Network::Testnet => Compact::from_u256(TESTNET_MAX_DIFFICULTY),
            Network::Alphanet => Compact::from_u256(ALPHA_MAX_DIFFICULTY),
            Network::Mainnet => Compact::from_u256(MAINNET_MAX_DIFFICULTY),
        }
    }
}
