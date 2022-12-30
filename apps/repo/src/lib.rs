use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::sync::Arc;

pub struct AppsRepository {
    apps: BTreeMap<u64, Arc<Executable>>,
    // TODO: install app by block height
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Metadata {
    pub activation_height: u64,
    pub publisher: String,
    pub docs: String,
    pub genesis_config: String,
    pub types_config: String,
    pub checksum: String,
}

#[derive(Encode, Decode, Serialize, Deserialize)]
pub struct Executable {
    pub binary: Vec<u8>,
    pub genesis: Vec<u8>,
    pub metadata: Metadata,
}

impl std::fmt::Debug for Executable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.metadata.fmt(f)
    }
}

impl AppsRepository {
    fn get_app(&self, id: u64) -> Option<&Arc<Executable>> {
        self.apps.get(&id)
    }

    fn install_app(&mut self, id: u64, exec: Executable) {
        self.apps.insert(id, Arc::new(exec));
    }
}
