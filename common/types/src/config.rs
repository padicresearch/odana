use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::network::Network;
use directories::UserDirs;
use primitive_types::address::Address;
use primitive_types::H256;

pub const DEFAULT_DIR_NAME: &str = ".odana";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeIdentityConfig {
    pub pub_key: H256,
    pub secret_key: H256,
    pub peer_id: String,
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
    pub miner: Option<Address>,
    #[serde(default)]
    pub p2p_host: String,
    pub p2p_port: u16,
    #[serde(default)]
    pub rpc_host: String,
    pub rpc_port: u16,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub peers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub identity_file: Option<PathBuf>,
    #[serde(default)]
    pub datadir: PathBuf,
    pub network: Network,
}

impl EnvironmentConfig {
    pub fn p2p_host(&self) -> &String {
        &self.p2p_host
    }
    pub fn rpc_host(&self) -> &String {
        &self.rpc_host
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
    pub fn datadir(&self) -> &PathBuf {
        &self.datadir
    }
    pub fn sanitize(&mut self) {
        let default = Self::default();
        if !self.datadir.exists() {
            self.datadir = default.datadir
        }
        if self.rpc_host.is_empty() {
            self.rpc_host = default.rpc_host
        }
        if self.p2p_host.is_empty() {
            self.p2p_host = default.p2p_host
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        let user_dir = UserDirs::new().unwrap();
        let mut default_datadir = PathBuf::from(user_dir.home_dir());
        default_datadir.push(DEFAULT_DIR_NAME);
        Self {
            miner: None,
            p2p_host: "0.0.0.0".to_string(),
            rpc_host: "127.0.0.1".to_string(),
            p2p_port: 9020,
            rpc_port: 9121,
            peers: vec![],
            identity_file: None,
            datadir: default_datadir,
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
