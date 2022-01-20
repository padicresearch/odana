use crate::error::Error;
use crate::miner_reward;
use chrono::Timelike;
use primitive_types::{Compact, H256, U256};
use std::sync::Arc;
use traits::{is_valid_proof_of_work, ChainHeadReader, Consensus, StateDB};
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::tx::Transaction;
use types::{Genesis, Hash};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Network {
    Testnet,
    Unitest,
}

pub struct BarossaProtocol {
    network: Network,
    max_difficulty: Compact,
}

impl BarossaProtocol {
    pub fn new(network: Network, max_difficulty: u32) -> Self {
        Self {
            network,
            max_difficulty: Compact::new(max_difficulty),
        }
    }
    pub fn max_difficulty(&self) -> &Compact {
        &self.max_difficulty
    }
}

impl BarossaProtocol {
    const MAX_POW_LIMIT: u32 = 503543726;
    /// Copy from https://github.com/mambisi/parity-bitcoin/blob/bf58a0d80ec196b99c9cf46b623b0a779af020f2/verification/src/work_bch.rs#L62
    /// Algorithm to adjust difficulty after each block. Implementation is based on Bitcoin ABC commit:
    /// https://github.com/Bitcoin-ABC/bitcoin-abc/commit/be51cf295c239ff6395a0aa67a3e13906aca9cb2
    fn calc_required_difficulty(
        &self,
        parent_header: IndexedBlockHeader,
        time: u32,
        height: u32,
        chain: Arc<dyn ChainHeadReader>,
    ) -> Compact {
        /// To reduce the impact of timestamp manipulation, we select the block we are
        /// basing our computation on via a median of 3.
        ///
        fn suitable_block(
            mut header2: IndexedBlockHeader,
            chain: Arc<dyn ChainHeadReader>,
        ) -> IndexedBlockHeader {
            let reason = "header.height >= RETARGETNG_INTERVAL; RETARGETING_INTERVAL > 2; qed";
            // let mut header1 = store.block_header(header2.raw.previous_header_hash.into()).expect(reason);
            // let mut header0 = store.block_header(header1.raw.previous_header_hash.into()).expect(reason);
            let mut header1 = chain
                .get_header_by_hash(&header2.raw.parent_hash)
                .unwrap()
                .expect(reason);
            let mut header0 = chain
                .get_header_by_hash(&header1.raw.parent_hash)
                .unwrap()
                .expect(reason);

            if header0.raw.time > header2.raw.time {
                std::mem::swap(&mut header0, &mut header2);
            }
            if header0.raw.time > header1.raw.time {
                std::mem::swap(&mut header0, &mut header1);
            }
            if header1.raw.time > header2.raw.time {
                std::mem::swap(&mut header1, &mut header2);
            }

            header1
        }

        /// Get block proof.
        fn block_proof(header: &IndexedBlockHeader) -> U256 {
            let proof: U256 = header.raw.difficulty().into();
            // We need to compute 2**256 / (bnTarget+1), but we can't represent 2**256
            // as it's too large for a arith_uint256. However, as 2**256 is at least as
            // large as bnTarget+1, it is equal to ((2**256 - bnTarget - 1) /
            // (bnTarget+1)) + 1, or ~bnTarget / (nTarget+1) + 1.
            (!proof / (proof + U256::one())) + U256::one()
        }

        /// Compute chain work between two blocks. Last block work is included. First block work is excluded.
        fn compute_work_between_blocks(
            first: H256,
            last: &IndexedBlockHeader,
            chain: Arc<dyn ChainHeadReader>,
        ) -> U256 {
            debug_assert!(last.hash != first);
            let mut chain_work: U256 = block_proof(last);
            let mut prev_hash = last.raw.parent_hash.clone();
            loop {
                let header = chain.get_header_by_hash(&prev_hash).unwrap()
                    .expect("last header is on main chain; first is at height last.height - 144; it is on main chain; qed");

                chain_work = chain_work + block_proof(&header);
                prev_hash = header.raw.parent_hash;
                if H256::from(prev_hash) == first {
                    return chain_work;
                }
            }
        }

        /// Compute the a target based on the work done between 2 blocks and the time
        /// required to produce that work.
        fn compute_target(
            first_header: IndexedBlockHeader,
            last_header: IndexedBlockHeader,
            chain: Arc<dyn ChainHeadReader>,
        ) -> U256 {
            // From the total work done and the time it took to produce that much work,
            // we can deduce how much work we expect to be produced in the targeted time
            // between blocks.
            let mut work = compute_work_between_blocks(first_header.hash, &last_header, chain);
            let c: U256 = U256::from(BarossaProtocol::TARGET_SPACING_SECONDS);
            work = work * c;

            // In order to avoid difficulty cliffs, we bound the amplitude of the
            // adjustement we are going to do.
            debug_assert!(last_header.raw.time > first_header.raw.time);
            let mut actual_timespan = last_header.raw.time - first_header.raw.time;
            if actual_timespan > 288 * BarossaProtocol::TARGET_SPACING_SECONDS {
                actual_timespan = 288 * BarossaProtocol::TARGET_SPACING_SECONDS;
            } else if actual_timespan < 72 * BarossaProtocol::TARGET_SPACING_SECONDS {
                actual_timespan = 72 * BarossaProtocol::TARGET_SPACING_SECONDS;
            }

            let work = work / U256::from(actual_timespan);

            // We need to compute T = (2^256 / W) - 1 but 2^256 doesn't fit in 256 bits.
            // By expressing 1 as W / W, we get (2^256 - W) / W, and we can compute
            // 2^256 - W as the complement of W.
            (!work) / work
        }

        // This cannot handle the genesis block and early blocks in general.
        debug_assert!(height > 0);

        // Special difficulty rule for testnet:
        // If the new block's timestamp is more than 2 * 10 minutes then allow
        // mining of a min-difficulty block.
        let max_bits = self.max_difficulty;
        if self.network == Network::Testnet || self.network == Network::Unitest {
            let max_time_gap = parent_header.raw.time + BarossaProtocol::DOUBLE_SPACING_SECONDS;
            if time > max_time_gap {
                return max_bits.into();
            }
        }

        // Compute the difficulty based on the full adjustement interval.
        let last_height = height - 1;
        debug_assert!(last_height >= Self::RETARGETING_INTERVAL);

        // Get the last suitable block of the difficulty interval.
        let last_header = suitable_block(parent_header, chain.clone());

        // Get the first suitable block of the difficulty interval.
        let first_height = last_height - 144;
        let first_header = chain
            .get_header_by_level(first_height as i32)
            .unwrap()
            .expect("last_height >= RETARGETING_INTERVAL; RETARGETING_INTERVAL - 144 > 0; qed");
        let first_header = suitable_block(first_header, chain.clone());

        // Compute the target based on time and work done during the interval.
        let next_target = compute_target(first_header, last_header, chain);
        let max_bits = self.max_difficulty.into();
        if next_target > max_bits {
            return max_bits.into();
        }

        next_target.into()
    }
}

impl Consensus for BarossaProtocol {
    const BLOCK_MAX_FUTURE: i64 = 2 * 60 * 60;
    const COINBASE_MATURITY: u32 = 100;
    const MIN_COINBASE_SIZE: usize = 2;
    const MAX_COINBASE_SIZE: usize = 100;
    const RETARGETING_FACTOR: u32 = 4;
    const TARGET_SPACING_SECONDS: u32 = 10 * 60;
    const DOUBLE_SPACING_SECONDS: u32 = 2 * Self::TARGET_SPACING_SECONDS;
    const TARGET_TIMESPAN_SECONDS: u32 = 2 * 7 * 24 * 60 * 60;

    fn verify_header(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &BlockHeader,
    ) -> anyhow::Result<()> {
        let current_time = chrono::Utc::now().timestamp();
        let _ = chain
            .get_header(&header.parent_hash, header.level - 1)?
            .ok_or(Error::ParentBlockNotFound)?;
        // Check timestamp
        anyhow::ensure!(
            (header.time as i64) < Self::BLOCK_MAX_FUTURE + current_time,
            "future block timestamp"
        );
        anyhow::ensure!(
            is_valid_proof_of_work(
                self.max_difficulty,
                header.difficulty(),
                &H256::from(header.hash())
            ),
            Error::BadPow
        );
        Ok(())
    }

    fn prepare_header(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &BlockHeader,
    ) -> anyhow::Result<BlockHeader> {
        todo!()
    }

    fn finalize(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &BlockHeader,
        state: Arc<dyn StateDB>,
        txs: Vec<Transaction>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn finalize_and_assemble(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &BlockHeader,
        state: Arc<dyn StateDB>,
        txs: Vec<Transaction>,
    ) -> anyhow::Result<Option<Block>> {
        todo!()
    }

    fn work_required(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        parent: &Hash,
        time: u32,
    ) -> anyhow::Result<Compact> {
        let parent_header = chain
            .get_header_by_hash(&parent)?
            .ok_or(Error::ParentBlockNotFound)?;
        let level = parent_header.raw.level as u32;
        Ok(self.calc_required_difficulty(parent_header, time, level, chain))
    }

    fn is_genesis(&self, header: &BlockHeader) -> bool {
        todo!()
    }

    fn miner_reward(&self, block_level: i32) -> u128 {
        miner_reward(block_level as u128)
    }
}
