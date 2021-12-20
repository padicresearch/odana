use crate::account::{Account, create_account};
use std::sync::Arc;
use crate::mempool::MemPool;
use crate::utxo::UTXO;
use crate::block::{BlockHeader, BlockTemplate};
use crate::block::Block;
use anyhow::Result;
use crate::transaction::Tx;
use crate::errors::BlockChainError;
use chrono::Utc;
use crate::consensus::check_block_pow;
use merkle::Merkle;


pub struct Miner {
    account: Account,
    mempool: Arc<MemPool>,
    utxo: Arc<UTXO>,
}


unsafe impl Send for Miner {}

unsafe impl Sync for Miner {}

impl Miner {
    pub fn new(mempool: Arc<MemPool>, utxo: Arc<UTXO>) -> Self {
        Self {
            account: create_account(),
            mempool,
            utxo,
        }
    }

    pub fn new_with_account(account: Account, mempool: Arc<MemPool>, utxo: Arc<UTXO>) -> Self {
        Self {
            account,
            mempool,
            utxo,
        }
    }
    pub fn mine(
        &self,
        current_block: &BlockHeader,
    ) -> Result<Block> {
        let mut txs = self.mempool.fetch()?;
        let mut fees: u128 = 0;
        for tx in txs.iter() {
            fees += crate::transaction::calculate_tx_in_out_amount(tx, self.utxo.as_ref()).map(
                |(in_amount, out_amount)| {
                    crate::consensus::check_transaction_fee(in_amount, out_amount)
                },
            )??;
            crate::consensus::validate_transaction(tx, self.utxo.as_ref())?;
        }

        txs.insert(0, Tx::coinbase(&self.account, fees)?);

        let mut merkle = Merkle::default();
        for tx in txs.iter() {
            let _ = merkle.update(tx.id())?;
        }
        let merkle_root = merkle.finalize().ok_or(BlockChainError::MerkleError)?;

        let mut nonce = 0;
        loop {
            let time = Utc::now().timestamp() as u32;

            let mut new_block_hash = [0_u8; 32];
            //self.current_nonce = rand::random();


            let template_block = BlockTemplate::new(
                current_block.level() + 1,
                nonce,
                *current_block.block_hash(),
                time,
                txs.len() as u16,
                *merkle_root,
            )?;
            let empty_block = [0_u8; 32];
            new_block_hash = template_block.block_hash();
            if new_block_hash != empty_block && check_block_pow(&new_block_hash) {
                let transactions: Vec<_> = txs.iter().map(|t| t.id().clone()).collect();
                return Ok(Block::new(template_block, transactions));
            }
            nonce += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utxo::UTXO;
    use crate::utxo::UTXOStore;
    use anyhow::Result;
    use std::sync::Arc;
    use storage::memstore::MemStore;
    use crate::mempool::MemPool;
    use std::time::Instant;
    use crate::block::{BlockHeader, genesis_block};
    use crate::miner::Miner;
    use crate::account::Account;
    use crate::transaction::Tx;

    pub struct TempStorage {
        pub utxo: Arc<UTXO>,
    }

    pub fn setup_storage(accounts: &Vec<Account>, memstore: Arc<MemStore>) -> TempStorage {
        let coin_base = [0_u8; 32];

        let res: Result<Vec<_>> = accounts
            .iter()
            .map(|account| Tx::coinbase(account, 0))
            .collect();

        let txs = res.unwrap();

        let temp = TempStorage {
            utxo: Arc::new(UTXO::new(memstore)),
        };

        for tx in txs.iter() {
            temp.utxo.put(tx).unwrap()
        }

        temp
    }

    #[test]
    fn mine_genesis() {
        let utxo = Arc::new(UTXO::new(Arc::new(MemStore::new())));
        let mempool = Arc::new(MemPool::new(utxo.clone(), Arc::new(MemStore::new()), None).unwrap());
        let mut miner = Miner::new(mempool.clone(), utxo.clone());
        let block = miner.mine(&BlockHeader::new(
            [0; 32],
            [0; 32],
            0,
            -1,
            0,
            [0; 32],
            0,
        )).unwrap();
        println!("{:?}", hex::encode(bincode::serialize(&block).unwrap()));
    }

    #[test]
    fn _genesis() {
        println!("{}", genesis_block());
    }

    #[test]
    fn test_miner() {
        let mut current_block = genesis_block().header();
        let utxo = Arc::new(UTXO::new(Arc::new(MemStore::new())));
        let mempool = Arc::new(MemPool::new(utxo.clone(), Arc::new(MemStore::new()), None).unwrap());
        let miner = Arc::new(Miner::new(mempool.clone(), utxo.clone()));
        println!("🔨 Genesis block:  {}", hex::encode(current_block.block_hash()));
        for i in 0..1 {
            let timer = Instant::now();
            let block = miner.mine(&current_block).unwrap();
            println!("🔨 Mined new block [{} secs]:  {}", timer.elapsed().as_secs(), hex::encode(block.hash()));
            current_block = block.header()
        }
    }
}
