use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use storage::PersistentStorage;
use tracing::{error, info};
use traits::Consensus;
use txpool::TxPool;
use types::events::LocalEventMessage;

use crate::block_storage::BlockStorage;
use crate::chain_state::{ChainState, ChainStateStorage};

pub struct Tuchain {
    chain: Arc<ChainState>,
    txpool: Arc<RwLock<TxPool>>,
}

impl Tuchain {
    pub fn initialize(
        dir: PathBuf,
        consensus: Arc<dyn Consensus>,
        main_storage: Arc<PersistentStorage>,
        lmpsc: UnboundedSender<LocalEventMessage>,
    ) -> Result<Self> {
        let chain_state_storage = Arc::new(ChainStateStorage::new(main_storage.database()));
        let block_storage = Arc::new(BlockStorage::new(main_storage));
        let chain_state = Arc::new(ChainState::new(
            dir.join("state"),
            consensus.clone(),
            block_storage,
            chain_state_storage,
        )?);
        let txpool = Arc::new(RwLock::new(TxPool::new(
            None,
            None,
            lmpsc,
            chain_state.clone(),
        )?));

        Ok(Tuchain {
            chain: chain_state,
            txpool,
        })
    }

    pub fn chain(&self) -> Arc<ChainState> {
        self.chain.clone()
    }

    pub fn txpool(&self) -> Arc<RwLock<TxPool>> {
        self.txpool.clone()
    }
}
