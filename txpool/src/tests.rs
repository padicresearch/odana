use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use dashmap::DashMap;

use account::create_account;
use primitive_types::{H160, H256};
use traits::{Blockchain, ChainHeadReader, ChainReader, StateDB};
use transaction::make_sign_transaction;
use types::account::{Account, AccountState};
use types::block::{Block, BlockHeader, IndexedBlockHeader};
use types::tx::SignedTransaction;
use types::Hash;

use crate::tx_lookup::AccountSet;
use crate::{ResetRequest, TransactionRef, TxPool};

#[derive(Clone)]
struct DummyStateDB {
    accounts: DashMap<H160, AccountState>,
}

pub fn make_tx(
    from: &Account,
    to: &Account,
    nonce: u64,
    amount: u128,
    fee: u128,
) -> TransactionRef {
    let tx = make_sign_transaction(
        from,
        nonce,
        to.address.to_fixed_bytes(),
        amount,
        fee,
        "".to_string(),
    )
    .unwrap();
    Arc::new(tx)
}

fn make_tx_def(
    from: &Account,
    to: &Account,
    nonce: u64,
    amount: u128,
    fee: u128,
) -> SignedTransaction {
    make_sign_transaction(
        from,
        nonce,
        to.address.to_fixed_bytes(),
        amount,
        fee,
        "".to_string(),
    )
    .unwrap()
}

impl DummyStateDB {
    fn with_accounts(accounts: Vec<(H160, AccountState)>) -> Self {
        let map = DashMap::new();
        for (addr, state) in accounts {
            map.insert(addr, state);
        }
        Self { accounts: map }
    }

    pub fn set_account_state(
        &self,
        address: H160,
        state: AccountState,
    ) -> Result<Option<AccountState>> {
        Ok(self.accounts.insert(address, state))
    }

    pub fn increment_nonce(&self, address: &H160, nonce: u64) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().nonce += nonce;
        *entry.value()
    }
    pub fn set_nonce(&self, address: &H160, nonce: u64) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().nonce = nonce;
        *entry.value()
    }

    pub fn set_balance(&self, address: &H160, amount: u128) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().free_balance = amount;
        *entry.value()
    }
}

#[derive(Clone)]
struct DummyChain {
    chain: Arc<RwLock<Vec<Block>>>,
    blocks: DashMap<H256, usize>,
    states: DashMap<H256, Arc<DummyStateDB>>,
}

impl DummyChain {
    fn new(blocks: Vec<Block>, inital_state: Arc<DummyStateDB>) -> Self {
        let c: DashMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(height, block)| (block.hash(), height))
            .collect();

        let map = DashMap::new();
        map.insert([0; 32].into(), inital_state);

        Self {
            chain: Arc::new(RwLock::new(blocks)),
            blocks: c,
            states: map,
        }
    }

    fn insert_state(&self, root: Hash, state: Arc<DummyStateDB>) {
        self.states.insert(root.into(), state.clone());
        self.states.insert([0; 32].into(), state);
    }

    fn add(&self, block: Block) {
        let mut chain = self.chain.write().unwrap();
        chain.push(block.clone());
        self.blocks.insert(block.hash(), chain.len() - 1);
    }
}

impl StateDB for DummyStateDB {
    fn nonce(&self, account_id: &H160) -> u64 {
        self.accounts
            .get(account_id)
            .map(|state| state.nonce as u64)
            .unwrap_or_default()
    }

    fn account_state(&self, account_id: &H160) -> AccountState {
        self.accounts
            .get(account_id)
            .map(|state| *state.value())
            .unwrap_or_default()
    }

    fn balance(&self, address: &H160) -> u128 {
        self.account_state(address).free_balance
    }

    fn credit_balance(&self, _address: &H160, _amount: u128) -> Result<H256> {
        todo!()
    }

    fn debit_balance(&self, _address: &H160, _amount: u128) -> Result<H256> {
        todo!()
    }

    fn reset(&self, _root: H256) -> Result<()> {
        todo!()
    }

    fn apply_txs(&self, _txs: Vec<SignedTransaction>) -> Result<H256> {
        todo!()
    }

    fn root(&self) -> Hash {
        todo!()
    }

    fn commit(&self) -> Result<()> {
        todo!()
    }

    fn snapshot(&self) -> Result<Arc<dyn StateDB>> {
        todo!()
    }

    fn state_at(&self, _root: H256) -> Result<Arc<dyn StateDB>> {
        todo!()
    }
}

impl Blockchain for DummyChain {
    fn get_current_state(&self) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(&H256::from([0; 32]))
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }

    fn current_header(&self) -> Result<Option<IndexedBlockHeader>> {
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.last().cloned().map(|b| (*b.header()).into());
        Ok(block)
    }

    fn get_state_at(&self, root: &H256) -> Result<Arc<dyn StateDB>> {
        let d = self
            .states
            .get(root)
            .ok_or(anyhow::anyhow!("no state found"))
            .map(|r| r.value().clone())?;
        Ok(d)
    }
}

impl ChainReader for DummyChain {
    fn get_block(&self, hash: &H256, _level: i32) -> Result<Option<Block>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block)
    }

    fn get_block_by_hash(&self, hash: &H256) -> Result<Option<Block>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block)
    }

    fn get_block_by_level(&self, _level: i32) -> Result<Option<Block>> {
        todo!()
    }
}

impl ChainHeadReader for DummyChain {
    fn get_header(&self, hash: &H256, _level: i32) -> Result<Option<IndexedBlockHeader>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block.map(|b| (*b.header()).into()))
    }

    fn get_header_by_hash(&self, hash: &H256) -> Result<Option<IndexedBlockHeader>> {
        let index = match self.blocks.get(hash) {
            None => return Ok(None),
            Some(block) => *block.value(),
        };
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(index).cloned();
        Ok(block.map(|b| (*b.header()).into()))
    }

    fn get_header_by_level(&self, level: i32) -> Result<Option<IndexedBlockHeader>> {
        let chain = self
            .chain
            .read()
            .map_err(|_e| anyhow::anyhow!("RW error"))?;
        let block = chain.get(level as usize).map(|bloc| *bloc.header());
        Ok(block.map(|header| header.into()))
    }
}

fn generate_blocks(n: usize) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::with_capacity(n);
    for level in 0..=n {
        let block = if blocks.is_empty() {
            make_block(
                level as i32,
                [0; 32].into(),
                H256::from_low_u64_be(level as u64),
            )
        } else {
            make_block(
                level as i32,
                blocks[level - 1].hash(),
                H256::from_low_u64_be(level as u64),
            )
        };
        blocks.push(block);
    }
    blocks
}

fn state_with_balance(amount: u128) -> AccountState {
    let mut state = AccountState::default();
    state.free_balance = amount;
    state
}

fn make_block(level: i32, parent_hash: H256, state_root: H256) -> Block {
    Block::new(
        BlockHeader::new(
            parent_hash,
            [0; 32].into(),
            state_root,
            [0; 32].into(),
            [0; 20].into(),
            0,
            0,
            level,
            0,
            0.into(),
        ),
        Vec::new(),
    )
}

#[tokio::test]
async fn txpool_test() {
    let alice = create_account();
    let bob = create_account();
    let state_db = Arc::new(DummyStateDB::with_accounts(vec![(
        alice.address,
        state_with_balance(1000),
    )]));
    let chain = Arc::new(DummyChain::new(Vec::new(), state_db.clone()));
    chain.insert_state([1; 32], state_db);
    chain.add(make_block(0, [0; 32].into(), [1; 32].into()));
    let (sender, _recv) = tokio::sync::mpsc::unbounded_channel();
    let mut txpool = TxPool::new(None, None, sender, chain.clone()).unwrap();
    txpool
        .add_txs(
            vec![
                make_tx_def(&alice, &bob, 1, 100, 10),
                make_tx_def(&alice, &bob, 2, 100, 10),
                make_tx_def(&alice, &bob, 3, 100, 10),
                make_tx_def(&alice, &bob, 4, 100, 10),
            ],
            true,
        )
        .unwrap();

    let state_db_1 = Arc::new(DummyStateDB::with_accounts(vec![(
        alice.address,
        state_with_balance(1000),
    )]));
    state_db_1.increment_nonce(&alice.address, 4);

    let block_2 = make_block(
        1,
        chain.current_header().unwrap().unwrap().hash,
        [2; 32].into(),
    );
    let old_head = chain.current_header().unwrap().unwrap().raw;
    let new_head = *block_2.header();
    chain.insert_state([2; 32], state_db_1);
    chain.add(block_2);
    txpool
        .repack(
            AccountSet::new(),
            Some(ResetRequest {
                old_head: Some(old_head),
                new_head,
            }),
        )
        .unwrap();

    txpool
        .add_txs(
            vec![
                make_tx_def(&alice, &bob, 5, 100, 10),
                make_tx_def(&alice, &bob, 6, 100, 10),
                make_tx_def(&alice, &bob, 7, 100, 10),
            ],
            true,
        )
        .unwrap();
    println!("{:#?}", txpool.nonce(&alice.address));
    println!("Pending {:#?}", txpool.pending);
}

// #[test]
// fn txpool_processing_speed_test() {
//     let alice = create_account();
//     let bob = create_account();
//     let state_db = Arc::new(DummyStateDB::with_accounts(vec![(alice.address, state_with_balance(1000000000000))]));
//     let block_1 = Block::new(
//         BlockTemplate::new(
//             0 as i32,
//             0 as u128,
//             [0; 32],
//             [0; 32],
//             0,
//             0,
//             [0; 32],
//             [1; 32],
//         )
//             .unwrap(),
//         Vec::new(),
//     );
//     let chain = Arc::new(DummyChain::new(Vec::new(), state_db.clone()));
//     chain.insert_state([1; 32], state_db.clone());
//     chain.add(block_1);
//     let (sender, mut recv) = tokio::sync::mpsc::unbounded_channel();
//     let mut txpool = TxPool::new(None, None, sender, chain.clone()).unwrap();
//     let mut tcount = 0;
//     let mut total_duration = Duration::new(0, 0);
//     let mut min_latency = u128::MAX;
//     let mut max_latency = 0;
//
//     for _ in 1..1000 {
//         let user = create_account();
//         state_db.set_balance(&user.address, 100000000000);
//         for i in 1..24 {
//             let tx = make_tx_def(&user, &alice, txpool.nonce(&alice.address) + i, i as u128 * 100, i as u128 * 10);
//             let instant = Instant::now();
//             txpool.add_txs(vec![tx], false);
//             let duration = instant.elapsed();
//             if duration.as_millis() < min_latency {
//                 min_latency = duration.as_millis();
//             }
//             if duration.as_millis() > max_latency {
//                 max_latency = duration.as_millis();
//             }
//             total_duration += duration;
//             tcount += 1;
//         }
//     }
//
//     // for nonce in 500..2000 {
//     //     let tx = make_tx_def(&alice, &bob, nonce, 300, 100);
//     //     let instant = Instant::now();
//     //     txpool.add_local(tx).unwrap();
//     //     let duration = instant.elapsed();
//     //     if duration.as_millis() < min_latency {
//     //         min_latency = duration.as_millis();
//     //     }
//     //     if duration.as_millis() > max_latency {
//     //         max_latency = duration.as_millis();
//     //     }
//     //     total_duration += duration;
//     //     tcount += 1;
//     // }
//
//     println!("Speed {} tx/sec, min latency {} ms, max latency {} ms", tcount as f64 / total_duration.as_secs_f64(), min_latency, max_latency);
//     //println!("Pending Count {} ", txpool.pending.get(&alice.address).unwrap().len() );
//     println!("Slots Count {} ", txpool.all.slots());
//     for (acc, list) in txpool.pending {
//         println!("{} {:?}", acc, list.len())
//     }
// }

//TODO add test from Ethereum TxPool Test

/// # Case1
/// Tests that when the pool reaches its global transaction limit, underpriced
/// transactions are gradually shifted out for more expensive ones and any gapped
/// pending transactions are moved into the queue.
///
/// Note, local transactions are never allowed to be dropped.

/// # Case 2
/// Tests that setting the transaction pool fee price to a higher value does not
/// remove local transactions.

/// # Case 3
/// Tests that setting the transaction pool fee to a higher value correctly
/// discards everything cheaper (legacy & dynamic fee) than that and moves any
/// gapped transactions back from the pending pool to the queue.
///
/// Note, local transactions are never allowed to be dropped.

/// # Case 4
/// Tests that setting the transaction pool fee to a higher value correctly
/// discards everything cheaper than that and moves any gapped transactions back
/// from the pending pool to the queue.
///
/// Note, local transactions are never allowed to be dropped.

/// # Case 5
/// Tests that if the transaction count belonging to multiple accounts go above
/// some hard threshold, if they are under the minimum guaranteed slot count then
/// the transactions are still kept.

/// # Case 6
/// Tests that if transactions start being capped, transactions are also removed from 'all'

/// # Case 7
/// Test the limit on transaction size is enforced correctly.
/// This test verifies every transaction having allowed size
/// is added to the pool, and longer transactions are rejected.

/// # Case 8
/// Tests that if the transaction count belonging to multiple accounts go above
/// some hard threshold, the higher transactions are dropped to prevent DOS
/// attacks.

/// # Case 9
/// Tests that even if the transaction count belonging to a single account goes
/// above some threshold, as long as the transactions are executable, they are
/// accepted.

/// # Case 10
/// Tests that if an account remains idle for a prolonged amount of time, any
/// non-executable transactions queued up are dropped to prevent wasting resources
/// on shuffling them around.
///
/// This logic should not hold for local transactions, unless the local tracking
/// mechanism is disabled.

/// # Case 11
/// Tests that if the transaction count belonging to multiple accounts go above
/// some threshold, the higher transactions are dropped to prevent DOS attacks.
///
/// This logic should not hold for local transactions, unless the local tracking
/// mechanism is disabled.

/// # Case 12
/// Tests that if the transaction count belonging to a single account goes above
/// some threshold, the higher transactions are dropped to prevent DOS attacks.

/// # Case 13
/// Tests that if the transaction pool has both executable and non-executable
/// transactions from an origin account, filling the nonce gap moves all queued
/// ones into the pending pool.

/// # Case 14
/// Tests that if a transaction is dropped from the current pending pool (e.g. out
/// of fund), all consecutive (still valid, but not executable) transactions are
/// postponed back into the future queue to prevent broadcasting them.

/// # Case 15
/// Tests that if an account runs out of funds, any pending and queued transactions
/// are dropped.
#[test]
fn test() {}
