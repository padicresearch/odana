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
    Alphanet,
    Mainnet,
}

const TESTNET_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x00000377ae000000u64,
]);
const ALPHA_MAX_DIFFICULTY: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x000000fff000000u64,
]);
const MAINNET_MAX_DIFFICULTY: U256 = U256([
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0xffffffffffffffffu64,
    0x00000000ffffffffu64,
]);

impl Network {
    pub fn max_difficulty(&self) -> U256 {
        match self {
            Network::Testnet => TESTNET_MAX_DIFFICULTY,
            Network::Alphanet => ALPHA_MAX_DIFFICULTY,
            Network::Mainnet => MAINNET_MAX_DIFFICULTY,
        }
    }

    pub fn max_difficulty_compact(&self) -> Compact {
        match self {
            Network::Testnet => Compact::from_u256(TESTNET_MAX_DIFFICULTY),
            Network::Alphanet => Compact::from_u256(ALPHA_MAX_DIFFICULTY),
            Network::Mainnet => Compact::from_u256(MAINNET_MAX_DIFFICULTY),
        }
    }
}

pub struct BarossaProtocol {
    network: Network,
}

impl BarossaProtocol {
    pub fn new(network: Network) -> Self {
        Self { network }
    }
}

impl BarossaProtocol {
    /// Copy from https://github.com/mambisi/parity-bitcoin/blob/bf58a0d80ec196b99c9cf46b623b0a779af020f2/verification/src/work_bch.rs#L62
    /// Algorithm to adjust difficulty after each block. Implementation is based on Bitcoin ABC commit:
    /// https://github.com/Bitcoin-ABC/bitcoin-abc/commit/be51cf295c239ff6395a0aa67a3e13906aca9cb2
    fn calc_required_difficulty(
        &self,
        parent_header: IndexedBlockHeader,
        time: u32,
        level: u32,
        chain: Arc<dyn ChainHeadReader>,
    ) -> Compact {
        /// To reduce the impact of timestamp manipulation, we select the block we are
        /// basing our computation on via a median of 3.
        ///
        fn suitable_block(
            mut header2: IndexedBlockHeader,
            chain: Arc<dyn ChainHeadReader>,
        ) -> IndexedBlockHeader {
            let reason = "header.level >= RETARGETNG_INTERVAL; RETARGETING_INTERVAL > 2; qed";
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
            let proof: U256 = header.raw.difficulty().to_u256().unwrap();
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
                    .expect("last header is on main chain; first is at level last.level - 144; it is on main chain; qed");

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
        debug_assert!(level > 0);

        // Special difficulty rule for testnet:
        // If the new block's timestamp is more than 2 * 10 minutes then allow
        // mining of a min-difficulty block.
        let max_bits: Compact = Compact::from_u256(self.network.max_difficulty());
        if self.network == Network::Testnet {
            let max_time_gap = parent_header.raw.time + BarossaProtocol::DOUBLE_SPACING_SECONDS;
            if time > max_time_gap {
                return max_bits.into();
            }
        }

        // Compute the difficulty based on the full adjustement interval.
        let last_level = level - 1;
        debug_assert!(last_level >= Self::RETARGETING_INTERVAL);

        // Get the last suitable block of the difficulty interval.
        let last_header = suitable_block(parent_header, chain.clone());

        // Get the first suitable block of the difficulty interval.
        let first_level = last_level - 144;
        let first_header = chain
            .get_header_by_level(first_level as i32)
            .unwrap()
            .expect("last_level >= RETARGETING_INTERVAL; RETARGETING_INTERVAL - 144 > 0; qed");
        let first_header = suitable_block(first_header, chain.clone());

        // Compute the target based on time and work done during the interval.
        let next_target = compute_target(first_header, last_header, chain);
        let max_bits = self.network.max_difficulty();
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
                self.network.max_difficulty().into(),
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
        Ok(self.calc_required_difficulty(parent_header, time, level + 1, chain))
    }

    fn is_genesis(&self, header: &BlockHeader) -> bool {
        todo!()
    }

    fn miner_reward(&self, block_level: i32) -> u128 {
        miner_reward(block_level as u128)
    }
}

#[cfg(test)]
mod tests {
    use crate::barossa::{BarossaProtocol, Network};
    use primitive_types::{Compact, H256, U256};
    use std::collections::HashMap;
    use traits::{ChainHeadReader, Consensus};
    use types::block::{BlockHeader, IndexedBlockHeader};
    use types::Hash;
    use std::sync::{Arc, RwLock};

    #[derive(Default)]
    struct MemoryBlockHeaderReader {
        pub by_height: RwLock<Vec<IndexedBlockHeader>>,
        pub by_hash: RwLock<HashMap<H256, usize>>,
    }

    impl MemoryBlockHeaderReader {
        pub fn insert(&self, header: BlockHeader) -> IndexedBlockHeader {
            let mut by_hash = self.by_hash.write().unwrap();
            let mut by_height = self.by_height.write().unwrap();

            let header: IndexedBlockHeader = header.into();
            by_hash.insert(header.hash, by_height.len());
            by_height.push(header.clone());
            header
        }
    }

    impl ChainHeadReader for MemoryBlockHeaderReader {
        fn current_header(&self) -> anyhow::Result<Option<IndexedBlockHeader>> {
            unimplemented!()
        }

        fn get_header(
            &self,
            hash: &Hash,
            level: i32,
        ) -> anyhow::Result<Option<IndexedBlockHeader>> {
            self.get_header_by_hash(hash)
        }

        fn get_header_by_hash(&self, hash: &Hash) -> anyhow::Result<Option<IndexedBlockHeader>> {
            let by_hash = self.by_hash.read().unwrap();
            Ok(by_hash
                .get(&H256::from(hash))
                .map(|h| {
                    let by_height = self.by_height.read().unwrap();
                    by_height[*h].clone()
                }))
        }

        fn get_header_by_level(&self, level: i32) -> anyhow::Result<Option<IndexedBlockHeader>> {
            let by_height = self.by_height.read().unwrap();
            Ok(by_height.get(level as usize).cloned())
        }
    }

    #[test]
    fn test_consensus_protocol() {
        let barossa = BarossaProtocol::new(Network::Mainnet);
    }

    #[test]
    fn test_consensus_protocol_adjusted_difficulty() {
        let barossa = BarossaProtocol::new(Network::Mainnet);


        let limit_bits = barossa.network.max_difficulty();
        let initial_bits = limit_bits >> 4;
        let initial_bits: Compact = limit_bits.into();
        let header_provider = Arc::new(MemoryBlockHeaderReader::default());

        // Genesis block.
        header_provider.insert(BlockHeader {
            parent_hash: [0; 32],
            merkle_root: [0; 32],
            state_root: [0; 32],
            mix_nonce: [0; 32],
            coinbase: [0; 20],
            difficulty: initial_bits.into(),
            chain_id: 0,
            level: 0,
            time: 1269211443,
            nonce: 0,
        });

        // Pile up some blocks every 10 mins to establish some history.
        for height in 1..2050 {
            let mut header = header_provider.get_header_by_level((height - 1).into()).unwrap().unwrap();
            header.raw.parent_hash = header.hash.into();
            header.raw.time = header.raw.time + 600;
            header.raw.level = height;
            header_provider.insert(header.raw);
        }

        // Difficulty stays the same as long as we produce a block every 10 mins.
        let header = header_provider.get_header_by_level(2049.into()).unwrap().unwrap();
        let current_bits = barossa.work_required(header_provider.clone(), header.hash.as_fixed_bytes(),
                                                 0).unwrap();
        for height in 2050..2060 {
            let mut header = header_provider.get_header_by_level((height - 1)).unwrap().unwrap();
            header.raw.parent_hash = header.hash.into();
            header.raw.time = header.raw.time + 600;
            header.raw.difficulty = current_bits.into();
            header.raw.level = header.raw.level + 1;
            header_provider.insert(header.raw);
            let parent = header_provider.get_header_by_level(height.into()).unwrap().unwrap();
            let calculated_bits = barossa.calc_required_difficulty(parent, 0, height as u32 + 1, header_provider.clone());
            debug_assert_eq!(calculated_bits, current_bits);
        }

        // Make sure we skip over blocks that are out of wack. To do so, we produce
        // a block that is far in the future
        let mut header = header_provider.get_header_by_level(2059.into()).unwrap().unwrap();
        header.raw.parent_hash = header.hash.into();
        header.raw.time = header.raw.time + 6000;
        header.raw.difficulty = current_bits.into();
        header_provider.insert(header.raw);
        let calculated_bits = barossa.calc_required_difficulty(header, 0, 2061, header_provider.clone());
        debug_assert_eq!(calculated_bits, current_bits);

        // .. and then produce a block with the expected timestamp.
        let mut header = header_provider.get_header_by_level(2060.into()).unwrap().unwrap();
        header.raw.parent_hash = header.hash.into();
        header.raw.time = header.raw.time + 2 * 600 - 6000;
        header.raw.difficulty = current_bits.into();
        header_provider.insert(header.raw);
        let calculated_bits = barossa.calc_required_difficulty(header_provider.get_header_by_level(2060).unwrap().unwrap(), 0, 2061, header_provider.clone());
        debug_assert_eq!(calculated_bits, current_bits);

        // // The system should continue unaffected by the block with a bogous timestamps.
        // for height in 2062..2082 {
        //     let mut header = header_provider.block_header((height - 1).into()).unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 600;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(height.into()).unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //     debug_assert_eq!(calculated_bits, current_bits);
        // }
        //
        // // We start emitting blocks slightly faster. The first block has no impact.
        // let mut header = header_provider.block_header(2081.into()).unwrap();
        // header.raw.previous_header_hash = header.hash;
        // header.raw.time = header.raw.time + 550;
        // header.raw.bits = current_bits;
        // header_provider.insert(header.raw);
        // let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(2082.into()).unwrap().into(),
        //                                                           0, 2083, &header_provider, &uahf_consensus);
        // debug_assert_eq!(calculated_bits, current_bits);
        //
        // // Now we should see difficulty increase slowly.
        // let mut current_bits = current_bits;
        // for height in 2083..2093 {
        //     let mut header = header_provider.block_header((height - 1).into()).unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 550;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(height.into()).unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //
        //     let current_work: U256 = current_bits.into();
        //     let calculated_work: U256 = calculated_bits.into();
        //     debug_assert!(calculated_work < current_work);
        //     debug_assert!((current_work - calculated_work) < (current_work >> 10));
        //
        //     current_bits = calculated_bits;
        // }
        //
        // // Check the actual value.
        // debug_assert_eq!(current_bits, 0x1c0fe7b1.into());
        //
        // // If we dramatically shorten block production, difficulty increases faster.
        // for height in 2093..2113 {
        //     let mut header = header_provider.block_header((height - 1).into()).unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 10;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(height.into()).unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //
        //     let current_work: U256 = current_bits.into();
        //     let calculated_work: U256 = calculated_bits.into();
        //     debug_assert!(calculated_work < current_work);
        //     debug_assert!((current_work - calculated_work) < (current_work >> 4));
        //
        //     current_bits = calculated_bits;
        // }
        //
        // // Check the actual value.
        // debug_assert_eq!(current_bits, 0x1c0db19f.into());
        //
        // // We start to emit blocks significantly slower. The first block has no
        // // impact.
        // let mut header = header_provider.block_header(2112.into()).unwrap();
        // header.raw.previous_header_hash = header.hash;
        // header.raw.time = header.raw.time + 6000;
        // header.raw.bits = current_bits;
        // header_provider.insert(header.raw);
        // let mut current_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(2113.into()).unwrap().into(),
        //                                                            0, 2114, &header_provider, &uahf_consensus);
        //
        // // Check the actual value.
        // debug_assert_eq!(current_bits, 0x1c0d9222.into());
        //
        // // If we dramatically slow down block production, difficulty decreases.
        // for height in 2114..2207 {
        //     let mut header = header_provider.block_header((height - 1).into()).unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 6000;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = barossa.(header_provider.get_header_by_level(height.into()).unwrap().unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //
        //     let current_work: U256 = current_bits.into();
        //     let calculated_work: U256 = calculated_bits.into();
        //     debug_assert!(calculated_work < limit_bits);
        //     debug_assert!(calculated_work > current_work);
        //     debug_assert!((calculated_work - current_work) < (current_work >> 3));
        //
        //     current_bits = calculated_bits;
        // }
        //
        // // Check the actual value.
        // debug_assert_eq!(current_bits, 0x1c2f13b9.into());
        //
        // // Due to the window of time being bounded, next block's difficulty actually
        // // gets harder.
        // let mut header = header_provider.get_header_by_level(2206.into()).unwrap().unwrap();
        // header.raw.previous_header_hash = header.hash;
        // header.raw.time = header.raw.time + 6000;
        // header.raw.bits = current_bits;
        // header_provider.insert(header.raw);
        // let mut current_bits = work_required_bitcoin_cash_adjusted(header_provider.get_header_by_level(2207.into()).unwrap().unwrap().into(),
        //                                                            0, 2208, &header_provider, &uahf_consensus);
        // debug_assert_eq!(current_bits, 0x1c2ee9bf.into());
        //
        // // And goes down again. It takes a while due to the window being bounded and
        // // the skewed block causes 2 blocks to get out of the window.
        // for height in 2208..2400 {
        //     let mut header = header_provider.get_header_by_level((height - 1).into()).unwrap().unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 6000;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(height.into()).unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //
        //     let current_work: U256 = current_bits.into();
        //     let calculated_work: U256 = calculated_bits.into();
        //     debug_assert!(calculated_work <= limit_bits);
        //     debug_assert!(calculated_work > current_work);
        //     debug_assert!((calculated_work - current_work) < (current_work >> 3));
        //
        //     current_bits = calculated_bits;
        // }
        //
        // // Check the actual value.
        // debug_assert_eq!(current_bits, 0x1d00ffff.into());
        //
        // // Once the difficulty reached the minimum allowed level, it doesn't get any
        // // easier.
        // for height in 2400..2405 {
        //     let mut header = header_provider.block_header((height - 1).into()).unwrap();
        //     header.raw.previous_header_hash = header.hash;
        //     header.raw.time = header.raw.time + 6000;
        //     header.raw.bits = current_bits;
        //     header_provider.insert(header.raw);
        //
        //     let calculated_bits = work_required_bitcoin_cash_adjusted(header_provider.block_header(height.into()).unwrap().into(),
        //                                                               0, height + 1, &header_provider, &uahf_consensus);
        //     debug_assert_eq!(calculated_bits, limit_bits.into());
        //
        //     current_bits = calculated_bits;
        // }
    }

    fn print_compact(target: Compact) {
        let target_32: u32 = target.into();
        println!("{}", target.to_f64());
        println!("{:?}", target_32);
        println!("{:?}", target);
    }
}