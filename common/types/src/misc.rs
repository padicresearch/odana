use crate::network::Network;

pub struct ChainInfo {
    chain: String,
    genesis_hash: Vec<u8>,
    difficulty: u32,
    network_difficulty: u32,
    blocks: u32,
}
