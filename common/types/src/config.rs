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
pub struct EnvironmentConfig {
    pub coinbase: H160,
    pub host: String,
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub data_dir: Option<String>,
    pub identity_file: Option<String>,
    pub peers: Vec<String>,
    pub bootnodes: Vec<String>,
    pub network: Network,
}


#[cfg(test)]
mod test {
    #[test]
    fn test_config() {}
}


