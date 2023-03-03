use core::cmp;
use std::sync::Arc;

use crypto::is_valid_proof_of_work;
use primitive_types::{Compact, H256, U256};
use smt::SparseMerkleTree;
use traits::{ChainHeadReader, Consensus, StateDB, WasmVMInstance};
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::network::Network;
use types::tx::SignedTransaction;

use crate::constants::{
    BLOCK_MAX_FUTURE, DOUBLE_SPACING_SECONDS, MAX_TIMESPAN, MIN_TIMESPAN, RETARGETING_INTERVAL,
    TARGET_SPACING_SECONDS, TARGET_TIMESPAN_SECONDS,
};
use crate::error::Error;
use crate::miner_reward;

pub const NODE_POW_TARGET: U256 = U256([
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x0000000000000000u64,
    0x00000fffff000000u64,
]);

pub struct BarossaProtocol {
    network: Network,
}

impl BarossaProtocol {
    pub fn new(network: Network) -> Self {
        Self { network }
    }
}

impl BarossaProtocol {
    /// Returns work required for given header
    pub fn work_required(
        &self,
        parent_hash: H256,
        time: u32,
        height: u32,
        chain: Arc<dyn ChainHeadReader>,
    ) -> Compact {
        let max_bits = self.network.max_difficulty_compact();
        if height == 0 {
            return max_bits;
        }

        let parent_header = chain
            .get_header_by_hash(&parent_hash)
            .unwrap()
            .expect("self.height != 0; qed");

        // match consensus.fork {
        //     ConsensusFork::BitcoinCash(ref fork) if height >= fork.height =>
        //         return work_required_bitcoin_cash(parent_header, time, height, store, consensus, fork, max_bits),
        //     _ => (),
        // }

        if self.is_retarget_height(height) {
            return self.work_required_retarget(parent_header, height, chain, max_bits);
        }

        if self.network == Network::Testnet {
            return self.work_required_testnet(parent_hash, time, height, chain);
        }

        parent_header.raw.difficulty()
    }

    pub fn is_retarget_height(&self, height: u32) -> bool {
        height % RETARGETING_INTERVAL == 0
    }

    fn range_constrain(&self, value: i64, min: i64, max: i64) -> i64 {
        cmp::min(cmp::max(value, min), max)
    }

    pub fn retarget_timespan(&self, retarget_timestamp: u32, last_timestamp: u32) -> u32 {
        // subtract unsigned 32 bit numbers in signed 64 bit space in
        // order to prevent underflow before applying the range constraint.
        let timespan = last_timestamp as i64 - retarget_timestamp as i64;
        self.range_constrain(timespan, MIN_TIMESPAN as i64, MAX_TIMESPAN as i64) as u32
    }

    /// Algorithm used for retargeting work every 2 weeks
    pub fn work_required_retarget(
        &self,
        parent_header: IndexedBlockHeader,
        height: u32,
        chain: Arc<dyn ChainHeadReader>,
        max_work_bits: Compact,
    ) -> Compact {
        let retarget_ref = height - RETARGETING_INTERVAL;
        let retarget_header = chain
            .get_header_by_level(retarget_ref)
            .unwrap()
            .expect("self.height != 0 && self.height % RETARGETING_INTERVAL == 0; qed");

        // timestamp of block(height - RETARGETING_INTERVAL)
        let retarget_timestamp = retarget_header.raw.time;
        // timestamp of parent block
        let last_timestamp = parent_header.raw.time;
        // bits of last block
        let last_bits = parent_header.raw.difficulty();

        let mut retarget: U256 = last_bits.into();
        let maximum: U256 = max_work_bits.into();

        retarget *= U256::from(self.retarget_timespan(retarget_timestamp, last_timestamp));
        retarget /= U256::from(TARGET_TIMESPAN_SECONDS);

        if retarget > maximum {
            max_work_bits
        } else {
            retarget.into()
        }
    }

    pub fn work_required_testnet(
        &self,
        parent_hash: H256,
        time: u32,
        height: u32,
        chain: Arc<dyn ChainHeadReader>,
    ) -> Compact {
        assert_ne!(
            height, 0,
            "cannot calculate required work for genesis block"
        );

        let mut bits = Vec::new();
        let mut block_ref: H256 = parent_hash;

        let parent_header = chain
            .get_header_by_hash(&block_ref)
            .unwrap()
            .expect("height != 0; qed");
        let max_time_gap = parent_header.raw.time + DOUBLE_SPACING_SECONDS;
        let max_bits = self.network.max_difficulty_compact();
        if time > max_time_gap {
            return max_bits;
        }

        // TODO: optimize it, so it does not make 2016!!! redundant queries each time
        for _ in 0..RETARGETING_INTERVAL {
            let previous_header = match chain.get_header_by_hash(&block_ref).unwrap() {
                Some(h) => h,
                None => {
                    break;
                }
            };
            bits.push(previous_header.raw.difficulty());
            block_ref = previous_header.raw.parent_hash;
        }

        for (index, bit) in bits.into_iter().enumerate() {
            if bit != max_bits || self.is_retarget_height(height - index as u32 - 1) {
                return bit;
            }
        }

        max_bits
    }
    #[allow(dead_code)]
    fn work_required_adjusted(
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
            let reason = "header.level >= RETARGETNG_INTERVAL; retargeting_interval > 2; qed";
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
            let mut prev_hash = last.raw.parent_hash;
            loop {
                let header = chain.get_header_by_hash(&prev_hash).unwrap()
                    .expect("last header is on main chain; first is at level last.level - 144; it is on main chain; qed");

                chain_work += block_proof(&header);
                prev_hash = header.raw.parent_hash;
                if prev_hash == first {
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
            let c: U256 = U256::from(TARGET_SPACING_SECONDS);
            work *= c;

            // In order to avoid difficulty cliffs, we bound the amplitude of the
            // adjustement we are going to do.
            debug_assert!(last_header.raw.time > first_header.raw.time);
            let mut actual_timespan = last_header.raw.time - first_header.raw.time;
            if actual_timespan > 288 * TARGET_SPACING_SECONDS {
                actual_timespan = 288 * TARGET_SPACING_SECONDS;
            } else if actual_timespan < 72 * TARGET_SPACING_SECONDS {
                actual_timespan = 72 * TARGET_SPACING_SECONDS;
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
            let max_time_gap = parent_header.raw.time + DOUBLE_SPACING_SECONDS;
            if time > max_time_gap {
                return max_bits;
            }
        }

        // Compute the difficulty based on the full adjustement interval.
        let last_level = level - 1;
        debug_assert!(last_level >= RETARGETING_INTERVAL);

        // Get the last suitable block of the difficulty interval.
        let last_header = suitable_block(parent_header, chain.clone());

        // Get the first suitable block of the difficulty interval.
        let first_level = last_level - 144;
        let first_header = chain
            .get_header_by_level(first_level)
            .unwrap()
            .expect("last_level >= retargeting_interval; retargeting_interval - 144 > 0; qed");
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
            (header.time as i64) < BLOCK_MAX_FUTURE + current_time,
            "future block timestamp"
        );
        anyhow::ensure!(
            is_valid_proof_of_work(
                self.network.max_difficulty().into(),
                header.difficulty(),
                &header.hash()
            ),
            Error::BadPow(self.network.max_difficulty().into(), header.difficulty())
        );
        Ok(())
    }

    fn prepare_header(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
    ) -> anyhow::Result<()> {
        let parent = chain
            .get_header(&header.parent_hash, header.level - 1)?
            .ok_or(Error::ParentBlockNotFound)?;
        header.chain_id = self.network.chain_id();
        header.difficulty = self
            .work_required(parent.hash, header.time, header.level + 1, chain)
            .into();
        Ok(())
    }

    fn finalize<'a>(
        &self,
        _chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        vm: Arc<dyn WasmVMInstance>,
        state: Arc<dyn StateDB>,
        txs: &[SignedTransaction],
    ) -> anyhow::Result<()> {
        let mut merkle = SparseMerkleTree::default();
        for tx in txs {
            merkle.update(tx.hash(), tx.hash())?;
        }
        state.apply_txs(vm, txs)?;
        let _ = state.credit_balance(&header.coinbase, self.miner_reward(header.level))?;
        state.commit()?;

        header.state_root = state.root();
        header.tx_root = merkle.root();
        Ok(())
    }

    fn finalize_and_assemble(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        header: &mut BlockHeader,
        vm: Arc<dyn WasmVMInstance>,
        state: Arc<dyn StateDB>,
        txs: &[SignedTransaction],
    ) -> anyhow::Result<Option<Block>> {
        self.finalize(chain, header, vm, state, &txs)?;
        let block = Block::new(*header, txs.into());
        Ok(Some(block))
    }

    fn work_required(
        &self,
        chain: Arc<dyn ChainHeadReader>,
        parent: &H256,
        time: u32,
    ) -> anyhow::Result<Compact> {
        let parent_header = chain
            .get_header_by_hash(parent)?
            .ok_or(Error::ParentBlockNotFound)?;
        Ok(self.work_required(parent_header.hash, time, parent_header.raw.level + 1, chain))
    }

    fn is_genesis(&self, _header: &BlockHeader) -> bool {
        todo!()
    }

    fn miner_reward(&self, block_level: u32) -> u64 {
        miner_reward(block_level as u128) as u64
    }

    fn get_genesis_header(&self) -> BlockHeader {
        BlockHeader::new(
            [0; 32].into(),
            [0; 32].into(),
            [0; 32].into(),
            [
                167, 166, 177, 200, 75, 77, 145, 25, 149, 154, 251, 233, 94, 46, 215, 162, 118, 43,
                119, 114, 196, 232, 42, 88, 209, 4, 27, 184, 193, 138, 143, 109,
            ]
            .into(),
            [0; 32].into(),
            [0; 44].into(),
            self.network.max_difficulty_compact().into(),
            self.network.chain_id(),
            0,
            0,
            0,
        )
    }

    fn network(&self) -> Network {
        self.network
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use chrono::Utc;

    use primitive_types::{Compact, ADDRESS_LEN, H256, U256};
    use traits::{ChainHeadReader, Consensus};
    use types::block::{BlockHeader, IndexedBlockHeader};

    use crate::barossa::{BarossaProtocol, Network};
    use crate::constants::{RETARGETING_INTERVAL, TARGET_SPACING_SECONDS};

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
        fn get_header(
            &self,
            hash: &H256,
            _level: u32,
        ) -> anyhow::Result<Option<IndexedBlockHeader>> {
            self.get_header_by_hash(hash)
        }

        fn get_header_by_hash(&self, hash: &H256) -> anyhow::Result<Option<IndexedBlockHeader>> {
            let by_hash = self.by_hash.read().unwrap();
            Ok(by_hash.get(hash).map(|h| {
                let by_height = self.by_height.read().unwrap();
                by_height[*h].clone()
            }))
        }

        fn get_header_by_level(&self, level: u32) -> anyhow::Result<Option<IndexedBlockHeader>> {
            let by_height = self.by_height.read().unwrap();
            Ok(by_height.get(level as usize).cloned())
        }
    }

    #[test]
    fn test_consensus_protocol() {
        let _barossa = BarossaProtocol::new(Network::Mainnet);
    }

    #[test]
    fn test_consensus_protocol_adjusted_difficulty() {
        let barossa = BarossaProtocol::new(Network::Mainnet);

        let limit_bits = barossa.network.max_difficulty();
        let _initial_bits = limit_bits >> 4;
        let initial_bits: Compact = limit_bits.into();
        let header_provider = Arc::new(MemoryBlockHeaderReader::default());

        // Genesis block.
        header_provider.insert(BlockHeader::new(
            [0; 32].into(),
            [0; 32].into(),
            [0; 32].into(),
            [
                167, 166, 177, 200, 75, 77, 145, 25, 149, 154, 251, 233, 94, 46, 215, 162, 118, 43,
                119, 114, 196, 232, 42, 88, 209, 4, 27, 184, 193, 138, 143, 109,
            ]
            .into(),
            [0; 32].into(),
            [0; ADDRESS_LEN].into(),
            initial_bits.into(),
            0,
            0,
            1269211443,
            0,
        ));

        // Pile up some blocks every 10 mins to establish some history.
        for height in 1..10251 {
            let mut header = header_provider
                .get_header_by_level(height - 1)
                .unwrap()
                .unwrap();

            header.raw.parent_hash = header.hash;
            header.raw.time = header.raw.time + 120;
            header.raw.level = height;
            header_provider.insert(header.raw);
        }

        // Difficulty stays the same as long as we produce a block every 10 mins.
        let header = header_provider
            .get_header_by_level(10251 - 1)
            .unwrap()
            .unwrap();
        let current_bits = barossa.work_required(
            header.hash,
            0,
            header.raw.level + 1,
            header_provider.clone(),
        );
        for height in 10251..10269 {
            let mut header = header_provider
                .get_header_by_level(height - 1)
                .unwrap()
                .unwrap();
            header.raw.parent_hash = header.hash;
            header.raw.time = header.raw.time + 120;
            header.raw.difficulty = current_bits.into();
            header.raw.level = header.raw.level + 1;
            header_provider.insert(header.raw);
            let parent = header_provider
                .get_header_by_level(height)
                .unwrap()
                .unwrap();
            let calculated_bits =
                barossa.work_required_adjusted(parent, 0, height + 1, header_provider.clone());
            debug_assert_eq!(calculated_bits, current_bits);
        }

        // Make sure we skip over blocks that are out of wack. To do so, we produce
        // a block that is far in the future
        let mut header = header_provider.get_header_by_level(10268).unwrap().unwrap();
        header.raw.parent_hash = header.hash;
        header.raw.time = header.raw.time + 1200;
        header.raw.difficulty = current_bits.into();
        header_provider.insert(header.raw);
        let calculated_bits =
            barossa.work_required_adjusted(header, 0, 10269, header_provider.clone());
        debug_assert_eq!(calculated_bits, current_bits);

        // .. and then produce a block with the expected timestamp.
        let mut header = header_provider.get_header_by_level(10269).unwrap().unwrap();
        header.raw.parent_hash = header.hash;
        header.raw.time = header.raw.time + 2 * 120 - 1200;
        header.raw.difficulty = current_bits.into();
        header_provider.insert(header.raw);
        let calculated_bits = barossa.work_required_adjusted(
            header_provider.get_header_by_level(10268).unwrap().unwrap(),
            0,
            10265,
            header_provider.clone(),
        );
        debug_assert_eq!(calculated_bits, current_bits);

        // The system should continue unaffected by the block with a bogous timestamps.
        for height in 10269..10296 {
            let mut header = header_provider
                .get_header_by_level((height - 1).into())
                .unwrap()
                .unwrap();
            header.raw.parent_hash = header.hash;
            header.raw.time = header.raw.time + 120;
            header.raw.difficulty = current_bits.into();
            header_provider.insert(header.raw);

            let parent = header_provider
                .get_header_by_level(height)
                .unwrap()
                .unwrap();
            let calculated_bits =
                barossa.work_required_adjusted(parent, 0, height + 1, header_provider.clone());
            debug_assert_eq!(calculated_bits, current_bits);
        }

        // We start emitting blocks slightly faster. The first block has no impact.
        let mut header = header_provider.get_header_by_level(10295).unwrap().unwrap();
        header.raw.parent_hash = header.hash;
        header.raw.time = header.raw.time + 100;
        header.raw.difficulty = current_bits.into();
        header_provider.insert(header.raw);
        let calculated_bits = barossa.work_required_adjusted(
            header_provider.get_header_by_level(10296).unwrap().unwrap(),
            0,
            10297,
            header_provider.clone(),
        );
        debug_assert_eq!(calculated_bits, current_bits);

        // Now we should see difficulty increase slowly.
        let mut current_bits = current_bits;
        for height in 10297..10301 {
            let mut header = header_provider
                .get_header_by_level((height - 1).into())
                .unwrap()
                .unwrap();
            header.raw.parent_hash = header.hash;
            header.raw.time = header.raw.time + 90;
            header.raw.difficulty = current_bits.into();
            header_provider.insert(header.raw);

            let parent = header_provider
                .get_header_by_level(height)
                .unwrap()
                .unwrap();
            let calculated_bits =
                barossa.work_required_adjusted(parent, 0, height + 1, header_provider.clone());

            let current_work: U256 = current_bits.into();
            let calculated_work: U256 = calculated_bits.into();
            println!("{current_work} {calculated_work}");
            // debug_assert!(calculated_work < current_work);
            // debug_assert!((current_work - calculated_work) < (current_work >> 10));

            current_bits = calculated_bits;
        }
    }
}
