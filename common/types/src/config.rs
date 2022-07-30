use anyhow::Result;
use derive_getters::Getters;
use std::env::temp_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use primitive_types::{H160, H192, H256, H448, U192};

use crate::network::Network;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeIdentityConfig {
    pub pub_key: H256,
    pub secret_key: H256,
    pub peer_id: String,
    pub nonce: U192,
    pub pow_stamp: H256,
}

impl NodeIdentityConfig {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<NodeIdentityConfig> {
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(&file);
        Ok(serde_json::from_reader(reader)?)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct EnvironmentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub miner: Option<H160>,
    pub host: String,
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub expected_pow: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub peers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub identity_file: Option<PathBuf>,
    pub network: Network,
}

impl EnvironmentConfig {
    pub fn host(&self) -> &String {
        &self.host
    }
    pub fn p2p_port(&self) -> u16 {
        self.p2p_port
    }
    pub fn rpc_port(&self) -> u16 {
        self.rpc_port
    }
    pub fn peers(&self) -> &Vec<String> {
        &self.peers
    }
    pub fn network(&self) -> Network {
        self.network
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            miner: None,
            host: "0.0.0.0".to_string(),
            p2p_port: 9020,
            rpc_port: 9121,
            expected_pow: 26.0,
            peers: vec![],
            identity_file: None,
            network: Network::Testnet,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::config::EnvironmentConfig;

    #[test]
    fn test_config() {
        let config = EnvironmentConfig::default();

        println!("{}", serde_json::to_string_pretty(&config).unwrap())
    }
}
