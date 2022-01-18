use std::iter::FromIterator;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use dashmap::DashMap;

use primitive_types::H160;
use traits::{ChainState, StateDB};
use types::account::{AccountState, Account};
use types::block::{Block, BlockHeader, BlockTemplate};
use types::Hash;
use crate::{TxPool, TransactionRef, ResetRequest};
use account::create_account;
use transaction::make_sign_transaction;
use types::tx::{TransactionKind, Transaction};
use std::rc::Rc;
use crate::tx_lookup::AccountSet;
use std::time::{Instant, Duration};

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
        TransactionKind::Transfer {
            from: from.pub_key,
            to: to.pub_key,
            amount,
            fee,
        },
    )
        .unwrap();
    Rc::new(tx)
}

fn make_tx_def(
    from: &Account,
    to: &Account,
    nonce: u64,
    amount: u128,
    fee: u128,
) -> Transaction {
    let tx = make_sign_transaction(
        from,
        nonce,
        TransactionKind::Transfer {
            from: from.pub_key,
            to: to.pub_key,
            amount,
            fee,
        },
    )
        .unwrap();
    tx
}

impl DummyStateDB {
    fn with_accounts(accounts: Vec<(H160, AccountState)>) -> Self {
        let mut map = DashMap::new();
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
        entry.value().clone()
    }
    pub fn set_nonce(&self, address: &H160, nonce: u64) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().nonce = nonce;
        entry.value().clone()
    }

    pub fn set_balance(&self, address: &H160, amount: u128) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().free_balance = amount;
        entry.value().clone()
    }
}

#[derive(Clone)]
struct DummyChain {
    chain: Arc<RwLock<Vec<Block>>>,
    blocks: DashMap<[u8; 32], usize>,
    states: DashMap<[u8; 32], Arc<DummyStateDB>>,
}

impl DummyChain {
    fn new(blocks: Vec<Block>, inital_state: Arc<DummyStateDB>) -> Self {
        let c: DashMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(height, block)| (block.hash(), height))
            .collect();

        let map = DashMap::new();
        map.insert([0; 32], inital_state);

        Self {
            chain: Arc::new(RwLock::new(blocks)),
            blocks: c,
            states: map,
        }
    }

    fn insert_state(&self, root: Hash, state: Arc<DummyStateDB>) {
        self.states.insert(root, state.clone());
        self.states.insert([0; 32], state);
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
            .map(|state| state.value().clone())
            .unwrap_or_default()
    }

    fn balance(&self, address: &H160) -> u128 {
        self.account_state(address).free_balance
    }
}

impl ChainState for DummyChain {
    fn current_head(&self) -> Result<BlockHeader> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        Ok(blocks.last().map(|block| block.header()).unwrap())
    }

    fn get_block(&self, block_hash: &Hash) -> Result<Option<Block>> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        let res = self
            .blocks
            .get(block_hash)
            .ok_or(anyhow::anyhow!("block not found"))?;
        let block_level = res.value().clone();
        Ok(blocks.get(block_level).cloned())
    }

    fn get_state_at(&self, root: &Hash) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(root)
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }

    fn get_current_state(&self) -> Result<Arc<dyn StateDB>> {
        let state = self
            .states
            .get(&[0; 32])
            .ok_or(anyhow::anyhow!("state not found"))?;
        let state = state.value().clone();
        Ok(state)
    }
}

fn generate_blocks(n: usize) -> Vec<Block> {
    let mut rng = rand::thread_rng();
    let mut blocks: Vec<Block> = Vec::with_capacity(n);
    for level in 0..=n {
        // let mut block_hash = [0u64; 2];
        // rng.fill(&mut block_hash);
        //let block_hash: [u8; 32] = rand::random();
        let block = if blocks.is_empty() {
            Block::new(
                BlockTemplate::new(
                    level as i32,
                    level as u128,
                    [0; 32],
                    [0; 32],
                    0,
                    0,
                    [0; 32],
                    [0; 32],
                )
                .unwrap(),
                Vec::new(),
            )
        } else {
            Block::new(
                BlockTemplate::new(
                    level as i32,
                    level as u128,
                    [0; 32],
                    blocks[level - 1].hash(),
                    0,
                    0,
                    [0; 32],
                    [0; 32],
                )
                    .unwrap(),
                Vec::new(),
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

#[tokio::test]
async fn txpool_test() {
    let alice = create_account();
    let bob = create_account();
    let state_db = Arc::new(DummyStateDB::with_accounts(vec![(alice.address, state_with_balance(1000))]));
    let block_1 = Block::new(
        BlockTemplate::new(
            0 as i32,
            0 as u128,
            [0; 32],
            [0; 32],
            0,
            0,
            [0; 32],
            [1; 32],
        )
            .unwrap(),
        Vec::new(),
    );
    let chain = Arc::new(DummyChain::new(Vec::new(), state_db.clone()));
    chain.insert_state([1; 32], state_db.clone());
    chain.add(block_1);
    let (sender, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let mut txpool = TxPool::new(None, None, sender, chain.clone()).unwrap();
    txpool.add_txs(vec![
        make_tx_def(&alice, &bob, 1, 100, 10),
        make_tx_def(&alice, &bob, 2, 100, 10),
        make_tx_def(&alice, &bob, 3, 100, 10),
        make_tx_def(&alice, &bob, 4, 100, 10),
    ], true);

    let state_db_1 = Arc::new(DummyStateDB::with_accounts(vec![(alice.address, state_with_balance(1000))]));
    state_db_1.increment_nonce(&alice.address, 4);


    let block_2 = Block::new(
        BlockTemplate::new(
            1 as i32,
            1 as u128,
            [0; 32],
            *chain.current_head().unwrap().block_hash(),
            0,
            0,
            [0; 32],
            [2; 32],
        )
            .unwrap(),
        Vec::new(),
    );


    let old_head = chain.current_head().unwrap();
    let new_head = block_2.header();
    chain.insert_state([2; 32], state_db_1);
    chain.add(block_2);
    txpool.repack(AccountSet::new(), Some(ResetRequest { old_head: Some(old_head), new_head })).unwrap();

    txpool.add_txs(vec![
        make_tx_def(&alice, &bob, 5, 100, 10),
        make_tx_def(&alice, &bob, 6, 100, 10),
        make_tx_def(&alice, &bob, 7, 100, 10),
    ], true);

    // txpool.add_local(make_tx_def(&alice,&bob,1, 100, 10)).unwrap();
    // state_db.set_nonce(&alice.address, 2);
    // txpool.add_local(make_tx_def(&alice,&bob,txpool.nonce(&alice.address), 100, 100)).unwrap();
    // txpool.add_local(make_tx_def(&alice,&bob,txpool.nonce(&alice.address), 100, 200)).unwrap();
    // txpool.add_local(make_tx_def(&alice,&bob,3, 100, 250)).unwrap();
    println!("{:#?}", txpool.nonce(&alice.address));
    println!("{:#?}", txpool.pending);
}

#[test]
fn txpool_processing_speed_test() {
    let alice = create_account();
    let bob = create_account();
    let state_db = Arc::new(DummyStateDB::with_accounts(vec![(alice.address, state_with_balance(1000000000000))]));
    let block_1 = Block::new(
        BlockTemplate::new(
            0 as i32,
            0 as u128,
            [0; 32],
            [0; 32],
            0,
            0,
            [0; 32],
            [1; 32],
        )
            .unwrap(),
        Vec::new(),
    );
    let chain = Arc::new(DummyChain::new(Vec::new(), state_db.clone()));
    chain.insert_state([1; 32], state_db.clone());
    chain.add(block_1);
    let (sender, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let mut txpool = TxPool::new(None, None, sender, chain.clone()).unwrap();
    let mut tcount = 0;
    let mut total_duration = Duration::new(0, 0);
    let mut min_latency = u128::MAX;
    let mut max_latency = 0;

    for _ in 1..1000 {
        let user = create_account();
        state_db.set_balance(&user.address, 100000000000);
        for i in 1..24 {
            let tx = make_tx_def(&user, &alice, txpool.nonce(&alice.address) + i, i as u128 * 100, i as u128 * 10);
            let instant = Instant::now();
            txpool.add_txs(vec![tx], false);
            let duration = instant.elapsed();
            if duration.as_millis() < min_latency {
                min_latency = duration.as_millis();
            }
            if duration.as_millis() > max_latency {
                max_latency = duration.as_millis();
            }
            total_duration += duration;
            tcount += 1;
        }
    }

    // for nonce in 500..2000 {
    //     let tx = make_tx_def(&alice, &bob, nonce, 300, 100);
    //     let instant = Instant::now();
    //     txpool.add_local(tx).unwrap();
    //     let duration = instant.elapsed();
    //     if duration.as_millis() < min_latency {
    //         min_latency = duration.as_millis();
    //     }
    //     if duration.as_millis() > max_latency {
    //         max_latency = duration.as_millis();
    //     }
    //     total_duration += duration;
    //     tcount += 1;
    // }

    println!("Speed {} tx/sec, min latency {} ms, max latency {} ms", tcount as f64 / total_duration.as_secs_f64(), min_latency, max_latency);
    //println!("Pending Count {} ", txpool.pending.get(&alice.address).unwrap().len() );
    println!("Slots Count {} ", txpool.all.slots());
    for (acc, list) in txpool.pending {
        println!("{} {:?}", acc, list.flatten().last().unwrap().price())
    }
}