use serde::{Deserialize, Serialize};

use primitive_types::{Compact, U256};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Testnet,
    Alphanet,
    Mainnet,
}

impl Into<String> for Network {
    fn into(self) -> String {
        match self {
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
const UNITNET_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000ffffff000000u64,
]);
const ALPHA_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x000000fff000000u64,
]);
const MAINNET_MAX_DIFFICULTY: U256 = U256([
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0x00000000ffffffffu64,
]);

impl Network {
    pub fn max_difficulty(&self) -> U256 {
        match self {
            Network::Testnet => TESTNET_MAX_DIFFICULTY,
            Network::Alphanet => ALPHA_MAX_DIFFICULTY,
            Network::Mainnet => MAINNET_MAX_DIFFICULTY,
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
