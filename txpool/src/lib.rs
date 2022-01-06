mod txlist;
mod tx_lookup;
mod tx_noncer;
mod error;

use types::TxHash;
use transaction::{validate_transaction};
use dashmap::{DashMap, ReadOnlyView};
use std::sync::{Arc, Mutex, PoisonError, MutexGuard};
use anyhow::{Result, Error};
use types::tx::Transaction;
use std::borrow::BorrowMut;
use std::ops::DerefMut;
use traits::{BlockchainState, StateDB};
use crate::tx_noncer::TxNoncer;
use crate::tx_lookup::TxLookup;

type TxHashRef = Arc<TxHash>;
type TransactionRef = Arc<Transaction>;

// TODO: truncate Pending transactions
#[derive(Clone)]
pub struct TxPoolConfig {
    transaction_limit: usize,
}

impl Default for TxPoolConfig {
    fn default() -> Self {
        TxPoolConfig {
            transaction_limit: 2048
        }
    }
}

pub struct TxPool<Chain, State> {
    chain: Chain,
    state: State,
    pending_nonces: TxNoncer<State>,
    lookup: TxLookup,
    config: TxPoolConfig,
}

pub type TxPoolIterator<'a> = Box<dyn 'a + Send + Iterator<Item=(TxHashRef, TransactionRef)>>;

impl<Chain, State> TxPool<Chain, State> where Chain: BlockchainState, State: StateDB {
    pub fn new(config: TxPoolConfig, chain: Chain, state: State) -> Result<Self> {
        Ok(Self {
            chain,
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup: TxLookup::new()?,
            config,
        })
    }

    #[cfg(test)]
    pub fn new_lookup(lookup: TxLookup, config: TxPoolConfig, chain: Chain, state: State) -> Result<Self> {
        Ok(Self {
            chain,
            state: state.clone(),
            pending_nonces: TxNoncer::new(state),
            lookup,
            config,
        })
    }

    pub fn add(&self, tx: Transaction, is_local: bool) -> Result<bool> {
        let tx_hash = Arc::new(tx.hash());
        let tx = Arc::new(tx);


        match validate_transaction(&tx, None, None) {
            Ok(_) => {}
            Err(error) => {
                return Err(error);
            }
        }

        if self.lookup.count() + 1 > self.config.transaction_limit {
            let old_tx = self.lookup.get_lowest_priced(tx.fees())?;
            match old_tx {
                None => {
                    println!("Discarding Tx {:?}", tx_hash)
                }
                Some(old_tx) => {
                    self.lookup.delete(&old_tx.hash())?;
                    self.lookup.add(tx_hash, tx, is_local)?;
                    return Ok(true);
                }
            }
        }

        let overlaping_tx = self.lookup.get_overlap_pending_tx(tx.sender_address(), tx.nonce_u32() as u64)?;
        if let Some(overlaping_tx) = overlaping_tx {
            let overlaping_tx_hash = overlaping_tx.hash();
            self.lookup.delete(&overlaping_tx_hash)?;
            // Add transaction to pending
            self.lookup.add(tx_hash.clone(), tx, is_local)?;
            //self.lookup.promote(vec![*tx_hash])?;
            return Ok(true);
        }
        // Add transaction to queue
        self.lookup.add(tx_hash, tx, is_local)?;
        Ok(false)
    }
    /// Takes transaction form queue and adds them to pending
    fn reorg(&self) {}

    pub fn remove_batch(&self, txs: Vec<TxHashRef>) {
        for tx_hash in txs.iter() {
            self.remove(tx_hash);
        }
    }

    /// Remove transaction form pending and queue
    /// This occurs when a new block
    pub fn remove(&self, tx_hash: &TxHash) {
        self.reorg()
    }

    // pub fn queue(&self) -> TxPoolIterator {
    //     Box::new(self.queue.iter().map(|kv| {
    //         (kv.key().clone(), kv.value().clone())
    //     }))
    // }
    //
    // pub fn pending(&self) -> TxPoolIterator {
    //     Box::new(self.pending.iter().map(|kv| {
    //         (kv.key().clone(), kv.value().clone())
    //     }))
    // }
}


#[cfg(test)]
mod test {
    use crate::{TxPool, TxPoolConfig};
    use transaction::make_sign_transaction;
    use account::create_account;
    use types::tx::TransactionKind;
    use primitive_types::H160;
    use std::collections::HashMap;
    use types::account::AccountState;
    use types::block::BlockHeader;
    use traits::{BlockchainState, StateDB};
    use dashmap::DashMap;
    use std::sync::{Arc, RwLock};
    use types::AccountId;
    use anyhow::Result;
    use rand::Rng;
    use std::iter::FromIterator;
    use crate::tx_lookup::TxLookup;

    #[derive(Clone)]
    struct DummyStateDB {
        accounts: DashMap<AccountId, AccountState>,
    }

    impl DummyStateDB {
        fn with_accounts(iter: Box<dyn Iterator<Item=(AccountId, AccountState)>>) -> Self {
            let mut accounts = DashMap::from_iter(iter);
            Self {
                accounts
            }
        }

        pub fn set_account_state(&mut self, account: AccountId, state: AccountState) -> Result<Option<AccountState>> {
            Ok(self.accounts.insert(account, state))
        }

        pub fn increment_nonce(&mut self, account: &AccountId, nonce: u64) -> AccountState {
            let mut entry = self.accounts.entry(*account).or_insert(AccountState::default());
            entry.value_mut().nonce += nonce as u32;
            entry.value().clone()
        }
        pub fn set_nonce(&mut self, account: &AccountId, nonce: u64) -> AccountState {
            let mut entry = self.accounts.entry(*account).or_insert(AccountState::default());
            entry.value_mut().nonce = nonce as u32;
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
            let c: DashMap<_, _> = blocks.iter().enumerate().map(|(height, block)| {
                (*block.block_hash(), height)
            }).collect();

            Self {
                chain: Arc::new(RwLock::new(blocks)),
                blocks: c,
            }
        }
    }

    impl StateDB for DummyStateDB {
        fn account_nonce(&self, account_id: &AccountId) -> u64 {
            self.accounts.get(account_id).map(|state|
                state.nonce as u64
            ).unwrap_or_default()
        }

        fn account_state(&self, account_id: &AccountId) -> AccountState {
            self.accounts.get(account_id).map(|state|
                state.value().clone()
            ).unwrap_or_default()
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
                BlockHeader::new([0; 32], block_hash, 0, level as i32, 0, [0; 32], level as u128)
            } else {
                BlockHeader::new(*blocks[level - 1].block_hash(), block_hash, 0, level as i32, 0, [0; 32], level as u128)
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

        let accounts = vec![(alice.pub_key, AccountState::default()), (bob.pub_key, AccountState::default())];
        let chain = DummyChain::new(generate_blocks(10));
        let state = DummyStateDB::with_accounts(Box::new(accounts.into_iter()));


        let txpool = TxPool::new_lookup(TxLookup::new_in_path("/home/mambisi/CLionProjects/tuchain/test/txpool.db").unwrap(), TxPoolConfig::default(), chain, state).unwrap();

        let tx1 = make_sign_transaction(&alice, 1, TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 10,
            fee: 0,
        }).unwrap();
        let tx2 = make_sign_transaction(&alice, 1, TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 1,
            fee: u128::MAX,
        }).unwrap();
        println!("{:}\n{:}", hex::encode(tx1.hash()), hex::encode(tx2.hash()));
        //txpool.add(, true).unwrap();
        txpool.add(tx1, true).unwrap();
        txpool.add(tx2, true).unwrap();
        //println!("{:?}", txpool)
    }
}