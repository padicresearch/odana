use anyhow::Result;
use chrono::Utc;
use merkle::Merkle;
use morph::Morph;
use primitive_types::{U256, H256, H160};
use std::rc::Rc;
use std::sync::atomic::AtomicI8;
use std::sync::{Arc, RwLock};
use traits::{Blockchain, ChainHeadReader, Consensus};
use txpool::TxPool;
use types::block::{BlockHeader, IndexedBlockHeader, Block};
use types::tx::Transaction;
use types::{Address, Hash};
use tracing::{trace, info, debug, warn, error};
use std::ops::Deref;
use crate::DummyChain;

pub fn start_worker(
    dummy: Arc<DummyChain>,
    coinbase: H160,
    consensus: Arc<dyn Consensus>,
    txpool: Arc<RwLock<TxPool>>,
    state: Arc<Morph>,
    chain: Arc<dyn ChainHeadReader>,
    interrupt: Arc<AtomicI8>,
) -> Result<()> {
    loop {
        println!("Starting miner Coinbase {}", coinbase);
        let (mut block_template, txs) = make_block_template(
            coinbase.to_fixed_bytes(),
            consensus.clone(),
            txpool.clone(),
            state.clone(),
            chain.clone(),
        )?;

        loop {
            if U256::from(block_template.nonce) + U256::one() > U256::from(u128::MAX) {
                let nonce = U256::from(block_template.nonce) + U256::one();
                let mut mix_nonce = [0_u8; 32];
                nonce.to_big_endian(&mut mix_nonce);
                block_template.mix_nonce = mix_nonce;
                block_template.nonce = 0
            }
            block_template.nonce += 1;
            if consensus.verify_header(chain.clone(), &block_template).is_ok() {
                let hash = block_template.hash();
                let level = block_template.level;
                println!("🔨 mined potential block number {} hash {:?}", level, hex::encode(hash));
                dummy.add(Block::new(block_template, txs));
                break;
            }
        }
    }
}

fn make_block_template(
    coinbase: Address,
    consensus: Arc<dyn Consensus>,
    txpool: Arc<RwLock<TxPool>>,
    state: Arc<Morph>,
    chain: Arc<dyn ChainHeadReader>,
) -> Result<(BlockHeader, Vec<Transaction>)> {
    let txpool = txpool.clone();
    let txpool = txpool.read().map_err(|e| anyhow::anyhow!("{}", e))?;
    let mut tsx = Vec::new();
    let mut merkle = Merkle::default();
    let mut state = state.intermediate()?;
    let mut logs = Vec::new();
    for (_, list) in txpool.pending() {
        for tx in list.iter() {
            merkle.update(&tx.hash());
            let log = state.apply_transaction(tx)?;
            logs.push(log);
        }
        tsx.extend(list.iter().map(|tx_ref| tx_ref.deref().clone()));
    }
    let merkle_root = match merkle.finalize() {
        None => {
            [0; 32]
        }
        Some(root) => {
            *root
        }
    };
    let state_root = state.root();
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
        state_root,
        mix_nonce,
        coinbase,
        difficulty: 0,
        chain_id: 0,
        level: parent_header.level + 1,
        time,
        nonce: 0,
    };

    consensus.prepare_header(chain, &mut header)?;
    Ok((header, tsx))
}
