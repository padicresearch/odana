use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};

use anyhow::Result;
use chrono::Utc;
use tokio::sync::mpsc::UnboundedSender;

use merkle::Merkle;
use primitive_types::{H160, H256, U256};
use tracing::{info, warn};
use traits::{ChainHeadReader, Consensus, StateDB};
use txpool::TxPool;
use types::Address;
use types::block::BlockHeader;
use types::events::LocalEventMessage;
use types::tx::Transaction;

pub const SHUTDOWN: i8 = -1;
pub const RESET: i8 = 0;
pub const PAUSE: i8 = 1;
pub const START: i8 = 2;

pub fn start_worker(
    coinbase: H160,
    lmpsc: UnboundedSender<LocalEventMessage>,
    consensus: Arc<dyn Consensus>,
    txpool: Arc<RwLock<TxPool>>,
    state: Arc<dyn StateDB>,
    chain: Arc<dyn ChainHeadReader>,
    interrupt: Arc<AtomicI8>,
) -> Result<()> {
    let is_running: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let mut current_block_template: Option<(BlockHeader, Vec<Transaction>)> = None;
    loop {
        let mut running = is_running.load(Ordering::Acquire);
        let i = interrupt.load(Ordering::Acquire);
        if i == SHUTDOWN {
            is_running.store(false, Ordering::Release);
            warn!(reason = i, "⛔ mine worker shutting down");
            return Ok(());
        } else if i == PAUSE {
            is_running.store(false, Ordering::Release);
            continue;
        } else if i == RESET {
            current_block_template = None;
            interrupt.store(START, Ordering::Release);
            continue;
        }

        if !running {
            is_running.store(true, Ordering::Release);
            info!(miner = ?coinbase, "👷 mine worker started running");
        }
        let (mut block_template, txs) = match &current_block_template {
            None => {
                let (head, txs) = make_block_template(
                    coinbase.to_fixed_bytes(),
                    consensus.clone(),
                    txpool.clone(),
                    state.clone(),
                    chain.clone(),
                )?;
                current_block_template = Some((head.clone(), txs.clone()));
                info!(coinbase = ?coinbase, txs_count = txs.len(), "🚧 mining a new block");
                (head, txs)
            }
            Some((head, txs)) => {
                (head.clone(), txs.clone())
            }
        };


        loop {
            let i = interrupt.load(Ordering::Acquire);
            if i == SHUTDOWN {
                break;
            } else if i == PAUSE {
                break;
            } else if i == RESET {
                break;
            }
            if U256::from(block_template.nonce) + U256::one() > U256::from(u128::MAX) {
                let nonce = U256::from(block_template.nonce) + U256::one();
                let mut mix_nonce = [0_u8; 32];
                nonce.to_big_endian(&mut mix_nonce);
                block_template.mix_nonce = mix_nonce;
                block_template.nonce = 0
            }
            block_template.nonce += 1;
            if consensus
                .verify_header(chain.clone(), &block_template)
                .is_ok()
            {
                let hash = block_template.hash();
                let level = block_template.level;
                info!(level = level, hash = ?hex::encode(hash), parent_hash = ?format!("{}", H256::from(block_template.parent_hash)), "⛏ mined potential block");
                // Apply the block to node state then broadcast it to other peers
                match consensus.finalize_and_assemble(chain.clone(), &mut block_template, state.clone(), txs)? {
                    None => {
                        warn!("Failed to finalize and assemble mined block");
                    }
                    Some(block) => {
                        lmpsc.send(LocalEventMessage::MindedBlock(block));
                    }
                }

                interrupt.store(RESET, Ordering::Release);

                break;
            }
        }
    }

    warn!("miner shutdown");
}

fn make_block_template(
    coinbase: Address,
    consensus: Arc<dyn Consensus>,
    txpool: Arc<RwLock<TxPool>>,
    state: Arc<dyn StateDB>,
    chain: Arc<dyn ChainHeadReader>,
) -> Result<(BlockHeader, Vec<Transaction>)> {
    let txpool = txpool.clone();
    let txpool = txpool.read().map_err(|e| anyhow::anyhow!("{}", e))?;
    let mut tsx = Vec::new();
    let mut merkle = Merkle::default();
    let mut state = state.snapshot()?;
    for (_, list) in txpool.pending() {
        for tx in list.iter() {
            merkle.update(&tx.hash());
        }
        tsx.extend(list.iter().map(|tx_ref| tx_ref.deref().clone()));
    }
    let merkle_root = match merkle.finalize() {
        None => [0; 32],
        Some(root) => *root,
    };
    let parent_header = match chain.current_header()? {
        None => consensus.get_genesis_header(),
        Some(header) => header.raw,
    };
    let mut mix_nonce = [0; 32];
    U256::one().to_big_endian(&mut mix_nonce);
    let time = Utc::now().timestamp() as u32;
    let mut header = BlockHeader {
        parent_hash: parent_header.hash(),
        merkle_root,
        state_root: [0; 32],
        mix_nonce,
        coinbase,
        difficulty: 0,
        chain_id: 0,
        level: parent_header.level + 1,
        time,
        nonce: 0,
    };
    consensus.prepare_header(chain.clone(), &mut header)?;
    consensus.finalize(chain, &mut header, state, tsx.clone())?;
    Ok((header, tsx))
}
