use crate::tx_lookup::TxLookup;
use crate::{TxPool, TxPoolConfig};
use account::create_account;
use anyhow::Result;
use dashmap::DashMap;
use primitive_types::H160;
use rand::Rng;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};
use traits::{BlockchainState, StateDB};
use transaction::make_sign_transaction;
use types::account::AccountState;
use types::block::BlockHeader;
use types::tx::TransactionKind;
use types::PubKey;

#[derive(Clone)]
struct DummyStateDB {
    accounts: DashMap<H160, AccountState>,
}

impl DummyStateDB {
    fn with_accounts(iter: Box<dyn Iterator<Item=(H160, AccountState)>>) -> Self {
        let mut accounts = DashMap::from_iter(iter);
        Self { accounts }
    }

    pub fn set_account_state(
        &mut self,
        address: H160,
        state: AccountState,
    ) -> Result<Option<AccountState>> {
        Ok(self.accounts.insert(address, state))
    }

    pub fn increment_nonce(&mut self, address: &H160, nonce: u64) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().nonce += nonce;
        entry.value().clone()
    }
    pub fn set_nonce(&mut self, address: &H160, nonce: u64) -> AccountState {
        let mut entry = self
            .accounts
            .entry(*address)
            .or_insert(AccountState::default());
        entry.value_mut().nonce = nonce;
        entry.value().clone()
    }
}

#[derive(Clone)]
struct DummyChain {
    chain: Arc<RwLock<Vec<BlockHeader>>>,
    blocks: DashMap<[u8; 32], usize>,
}

impl DummyChain {
    fn new(blocks: Vec<BlockHeader>) -> Self {
        let c: DashMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(height, block)| (*block.block_hash(), height))
            .collect();

        Self {
            chain: Arc::new(RwLock::new(blocks)),
            blocks: c,
        }
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

impl BlockchainState for DummyChain {
    fn current_head(&self) -> anyhow::Result<Option<BlockHeader>> {
        let blocks = self.chain.read().map_err(|_| anyhow::anyhow!("RW error"))?;
        Ok(blocks.last().map(|block| block.clone()))
    }
}

fn generate_blocks(n: usize) -> Vec<BlockHeader> {
    let mut rng = rand::thread_rng();
    let mut blocks: Vec<BlockHeader> = Vec::with_capacity(n);
    for level in 0..=n {
        let mut block_hash = [0u64; 2];
        rng.fill(&mut block_hash);
        let block_hash: [u8; 32] = rand::random();
        let block = if blocks.is_empty() {
            BlockHeader::new(
                [0; 32],
                block_hash,
                0,
                level as i32,
                0,
                [0; 32],
                level as u128,
            )
        } else {
            BlockHeader::new(
                *blocks[level - 1].block_hash(),
                block_hash,
                0,
                level as i32,
                0,
                [0; 32],
                level as u128,
            )
        };
        blocks.push(block);
    }
    blocks
}

use codec::Encoder;

#[test]
fn test_txpool() {
    let alice = create_account();
    let bob = create_account();

    let accounts = vec![
        (alice.address, AccountState::default()),
        (bob.address, AccountState::default()),
    ];
    let chain = DummyChain::new(generate_blocks(10));
    let state = DummyStateDB::with_accounts(Box::new(accounts.into_iter()));

    let txpool = TxPool::new_lookup(
        TxLookup::new_in_path("/home/mambisi/CLionProjects/tuchain/test/txpool.db").unwrap(),
        TxPoolConfig::default(),
        chain,
        state,
    )
        .unwrap();

    let tx1 = make_sign_transaction(
        &alice,
        1,
        TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 10,
            fee: 0,
        },
    )
        .unwrap();
    let tx2 = make_sign_transaction(
        &alice,
        1,
        TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 1,
            fee: u128::MAX,
        },
    )
        .unwrap();
    println!("{:}\n{:}", hex::encode(tx1.hash()), hex::encode(tx2.hash()));
    //txpool.add(, true).unwrap();
    txpool.add(tx1, true).unwrap();
    txpool.add(tx2, true).unwrap();
    //println!("{:?}", txpool)
}