use std::env::temp_dir;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use primitive_types::{H160, H256, U192};

use crate::network::Network;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeIdentityConfig {
    pub pub_key: H256,
    pub secret_key: H256,
    pub peer_id: String,
    pub nonce: U192,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct EnvironmentConfig {
    pub coinbase: String,
    pub host: String,
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub data_dir: PathBuf,
    pub identity_file: PathBuf,
    pub peers: Vec<String>,
    pub bootnodes: Vec<String>,
    pub network: Network,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            coinbase: Default::default(),
            host: "0.0.0.0".to_string(),
            p2p_port: 9020,
            rpc_port: 9121,
            data_dir: temp_dir().join("tuchain"),
            identity_file: temp_dir().join("tuchain").join("identity.json"),
            peers: vec![],
            bootnodes: vec![],
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
