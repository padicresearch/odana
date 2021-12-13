use crate::account::{create_account, Account};
use crate::consensus::validate_transaction;
use crate::errors::BlockChainError;
use crate::transaction::{Tx, TxIn, TxOut};
use crate::utxo::UTXO;
use anyhow::Result;
use chrono::Utc;
use itertools::Itertools;
use merkle::{Merkle, MerkleRoot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tiny_keccak::Hasher;
use std::fmt::Formatter;
use crate::mempool::MemPool;

pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MinerPubKey = [u8; 32];
pub type MinerSig = [u8; 64];
pub type MerkleHash = [u8; 32];

pub const BLOCK_DIFFICULTY: &str = "0000";
pub const GENESIS_BLOCK: &str = "0000006c84fcf7baaeefea10dff32bb1a77fcdf29d31e6edaf2a39a48730ad18000000000000000000000000000000000000000000000000000000000000000000c4b461010000fdd7eaabe9ad12278e4336b162c03903c36adf46aff1640c280920f1f4764ad9df4cd1c9e0597b28f85ab9aee8be2201000000000000001a18338c06a3cdfa0394efe757b9054a0fa9a05e61c4f057f4d19ca0fae65090";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct BlockHeader {
    prev_block_hash: BlockHash,
    block_hash: BlockHash,
    time: u32,
    tx_count: u16,
    merkle_root: MerkleHash,
    nonce: u128,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct BlockTemplate {
    prev_block_hash: BlockHash,
    time: u32,
    tx_count: u16,
    merkle_root: MerkleHash,
    nonce: u128,
}

impl BlockTemplate {
    pub fn new(
        nonce: u128,
        prev_block_hash: BlockHash,
        time: u32,
        tx_count: u16,
        merkle_root: MerkleHash,
    ) -> Result<Self> {
        Ok(Self {
            prev_block_hash,
            time,
            tx_count,
            merkle_root,
            nonce,
        })
    }

    pub fn block_hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.prev_block_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.nonce.to_be_bytes());
        sha3.update(&self.tx_count.to_be_bytes());
        sha3.update(&self.time.to_be_bytes());
        sha3.finalize(&mut block_hash);
        block_hash
    }
}

pub fn genesis_block() -> Block {
    let decoded_bytes = hex::decode(GENESIS_BLOCK).expect("Error creating genesis block");
    let genesis_block: Block =
        bincode::deserialize(&decoded_bytes).expect("Error creating genesis block");
    genesis_block
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub hash: BlockHash,
    pub prev_block_hash: BlockHash,
    pub time: u32,
    pub tx_count: u16,
    pub nonce: u128,
    pub merkle_root: MerkleHash,
    pub transactions: Vec<TxHash>,
}

#[derive(Debug)]
pub struct BlockView {
    block_hash: String,
    prev_block_hash: String,
    time: u32,
    tx_count: u16,
    nonce: u128,
    merkle_root: String,
    transactions: Vec<String>,
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let view = BlockView {
            block_hash: hex::encode(self.hash),
            prev_block_hash: hex::encode(self.prev_block_hash),
            time: self.time,
            tx_count: self.tx_count,
            nonce: self.nonce,
            merkle_root: hex::encode(self.merkle_root),
            transactions: self.transactions.iter().map(|tx| hex::encode(tx)).collect(),
        };
        write!(f, "{:#?}", view)
    }
}

impl Block {
    pub fn new(template: BlockTemplate, transactions: Vec<TxHash>) -> Self {
        Self {
            hash: template.block_hash(),
            prev_block_hash: template.prev_block_hash,
            time: template.time,
            tx_count: template.tx_count,
            nonce: template.nonce,
            merkle_root: template.merkle_root,
            transactions,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut block_hash = [0_u8; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(&self.prev_block_hash);
        sha3.update(&self.merkle_root);
        sha3.update(&self.nonce.to_be_bytes());
        sha3.update(&self.tx_count.to_be_bytes());
        sha3.update(&self.time.to_be_bytes());
        sha3.finalize(&mut block_hash);
        block_hash
    }

    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            prev_block_hash: self.prev_block_hash,
            block_hash: self.hash,
            time: self.time,
            tx_count: self.tx_count,
            merkle_root: self.merkle_root,
            nonce: self.nonce,
        }
    }
}

pub struct Miner {
    current_nonce: u128,
    account: Account,
    mempool: Arc<MemPool>,
    utxo: Arc<UTXO>,
}

fn validate_block(block_hash: &BlockHash) -> bool {
    let block_hash_encoded = hex::encode(block_hash);
    block_hash_encoded.starts_with(BLOCK_DIFFICULTY)
}

impl Miner {
    pub fn new(mempool: Arc<MemPool>, utxo: Arc<UTXO>) -> Self {
        Self {
            current_nonce: 0,
            account: create_account(),
            mempool,
            utxo
        }
    }
    pub fn mine(
        &mut self,
        prev_block_hash: &BlockHash,
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

        //let fee : u128 = txs.iter().map(|tx| {crate::consensus::calculate_fees(tx,utxo.as_ref())}).fold_ok(0, |acc, curr| acc + curr)?;

        txs.insert(0, Tx::coinbase(&self.account, fees)?);

        let mut merkle = Merkle::default();
        for tx in txs.iter() {
            let _ = merkle.update(tx.id())?;
        }
        let merkle_root = merkle.finalize().ok_or(BlockChainError::MerkleError)?;

        loop {
            let time = Utc::now().timestamp() as u32;

            let mut new_block_hash = [0_u8; 32];
            self.current_nonce = rand::random();

            let template_block = BlockTemplate::new(
                self.current_nonce,
                *prev_block_hash,
                time,
                txs.len() as u16,
                *merkle_root,
            )?;
            let empty_block = [0_u8; 32];
            new_block_hash = template_block.block_hash();
            if new_block_hash != empty_block && validate_block(&new_block_hash) {
                let transactions: Vec<_> = txs.iter().map(|t| t.id().clone()).collect();
                return Ok(Block::new(template_block, transactions));
            }
        }
    }
}

pub struct BlockChain {
    chain: Vec<BlockHeader>,
    blocks: HashMap<BlockHeader, Block>,
}

#[cfg(test)]
mod test {
    use crate::blockchain::{Account, genesis_block};
    use crate::blockchain::Tx;
    use crate::blockchain::{Miner};
    use crate::utxo::UTXO;
    use crate::utxo::UTXOStore;
    use anyhow::Result;
    use std::sync::Arc;
    use storage::memstore::MemStore;

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

    /*#[test]
    fn mine_genesis() {

        let utxo = Arc::new(UTXO::new(Arc::new(MemStore::new())));
        let mut miner = Miner::new();
        let block = miner.mine(&[0; 32], vec![], utxo.clone()).unwrap();
        println!("{:?}", hex::encode(bincode::serialize(&block).unwrap()));
    }*/

    #[test]
    fn _genesis() {
        println!("{}", genesis_block());
    }

    /*#[test]
    fn test_miner() {
        let mut prev_block = Block::genesis_block().header();
        let mut miner = Miner::new();
        println!("🔨 Genesis block:  {}", hex::encode(prev_block.block_hash));
        for i in 0..5 {
            let timer = Instant::now();
            let block = miner.mine_block(&prev_block.block_hash, vec![]).unwrap().header();
            println!("🔨 Mined new block [{} secs]:  {}", timer.elapsed().as_secs(), hex::encode(block.block_hash));
            prev_block = block
        }
    }*/
}
