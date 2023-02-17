use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use storage::PersistentStorage;
use traits::Consensus;
use txpool::TxPool;
use types::events::LocalEventMessage;

use crate::block_storage::BlockStorage;
use crate::chain_state::{ChainState, ChainStateStorage};

pub struct Chain {
    chain: Arc<ChainState>,
    txpool: Arc<RwLock<TxPool>>,
}

impl Chain {
    pub fn initialize(
        dir: PathBuf,
        consensus: Arc<dyn Consensus>,
        main_storage: Arc<PersistentStorage>,
        lmpsc: UnboundedSender<LocalEventMessage>,
        built_in_apps: Vec<(&'static str, &[u8])>,
    ) -> Result<Self> {
        let chain_state_storage = Arc::new(ChainStateStorage::new(main_storage.database()));
        let block_storage = Arc::new(BlockStorage::new(main_storage));
        let chain_state = Arc::new(ChainState::new(
            dir.join("state"),
            consensus.clone(),
            block_storage,
            built_in_apps,
            chain_state_storage,
            lmpsc.clone(),
        )?);
        let txpool = Arc::new(RwLock::new(TxPool::new(
            None,
            None,
            lmpsc,
            chain_state.clone(),
        )?));

        Ok(Chain {
            chain: chain_state,
            txpool,
        })
    }

    pub fn chain_state(&self) -> Arc<ChainState> {
        self.chain.clone()
    }

    pub fn txpool(&self) -> Arc<RwLock<TxPool>> {
        self.txpool.clone()
    }
}
