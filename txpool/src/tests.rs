use std::collections::{BTreeSet, HashMap, HashSet};
use std::env;
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use dashmap::DashMap;
use rand::Rng;

use account::create_account;
use primitive_types::H160;
use traits::{ChainState, StateDB};
use transaction::make_sign_transaction;
use types::{Hash, PubKey};
use types::account::AccountState;
use types::block::{Block, BlockHeader, BlockTemplate};
use types::tx::TransactionKind;

use crate::{TxPool, TxPoolConfig};
use crate::tx_lookup::TxLookup;

#[derive(Clone)]
struct DummyStateDB {
    accounts: DashMap<H160, AccountState>,
}

impl DummyStateDB {
    fn with_accounts(iter: Box<dyn Iterator<Item = (H160, AccountState)>>) -> Self {
        let mut accounts = DashMap::from_iter(iter);
        Self { accounts }
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
    fn new(blocks: Vec<Block>) -> Self {
        let c: DashMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(height, block)| (*block.hash(), height))
            .collect();

        Self {
            chain: Arc::new(RwLock::new(blocks)),
            blocks: c,
            states: Default::default(),
        }
    }

    fn insert_state(&self, root: Hash, state: Arc<DummyStateDB>) {
        self.states.insert(root, state);
    }
}

impl StateDB for DummyStateDB {
    fn account_nonce(&self, account_id: &H160) -> u64 {
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
                BlockTemplate::new(level as i32, level as u128, [0; 32], 0, 0, [0; 32], [0; 32])
                    .unwrap(),
                Vec::new(),
            )
        } else {
            Block::new(
                BlockTemplate::new(
                    level as i32,
                    level as u128,
                    *blocks[level - 1].hash(),
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

#[test]
fn test_txpool() {
    let alice = create_account();
    let bob = create_account();

    let accounts = vec![
        (alice.address, AccountState::default()),
        (bob.address, AccountState::default()),
    ];

    let chain = Arc::new(DummyChain::new(generate_blocks(10)));
    let state = Arc::new(DummyStateDB::with_accounts(Box::new(accounts.into_iter())));
    state.set_balance(&alice.address, 1000);
    state.set_balance(&bob.address, 1000);
    let test_dir = env::var("TEST_DIR").unwrap();

    let mut txpool = TxPool::new_lookup(
        TxLookup::new_in_path(format!("{}/{}", test_dir, "txpool.db")).unwrap(),
        TxPoolConfig::default(),
        chain.clone(),
        state.clone(),
    )
    .unwrap();

    state.set_nonce(&alice.address, 2);

    let tx1 = make_sign_transaction(
        &alice,
        3,
        TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 10,
            fee: 10,
        },
    )
    .unwrap();
    let tx2 = make_sign_transaction(
        &bob,
        3,
        TransactionKind::Transfer {
            from: bob.pub_key,
            to: alice.pub_key,
            amount: 100,
            fee: 0,
        },
    )
    .unwrap();

    let mut txs = HashSet::new();
    txs.insert(tx1.clone());
    txs.insert(tx2.clone());

    assert_eq!(txs.len(), 2);

    let tx2_hash = tx2.hash();

    println!("{:}\n{:}", hex::encode(tx1.hash()), hex::encode(tx2.hash()));
    txpool.add_local(tx1.clone()).unwrap();
    txpool.add_local(tx2.clone()).unwrap();
    println!("Stats: {:?}", txpool.package().unwrap().len())
    //println!("{:?}", txpool)
}
