use std::collections::HashMap;
use crate::transaction::{Tx, TxIn, TxOut};
use crate::account::{Account, create_account};
use anyhow::Result;
use merkle::{Merkle, MerkleRoot};
use crate::errors::BlockChainError;
use tiny_keccak::Hasher;
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MinerPubKey = [u8; 32];
pub type MinerSig = [u8; 64];
pub type MerkleHash = [u8; 32];

pub const BLOCK_DIFFICULTY: &str = "00000";
pub const GENESIS_BLOCK: &str = "00000000000000000000000000000000000000000000000000000000000000005ae99a6101002dd70000000000000000000000000000dd0808c02e8a54374128d3d7b8c579c74753f1c89b2c6c2473157a9db4eac730010000000000000034c9605c033fd884d2b11759bfc7de3f469d2b977bbeb4904e018898fddfb80e";

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
    pub fn new(nonce: u128, prev_block_hash: BlockHash, time: u32, tx_count: u16, merkle_root : MerkleHash) -> Result<Self> {


        Ok(Self {
            prev_block_hash,
            time,
            tx_count,
            merkle_root ,
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
    let genesis_block: Block = bincode::deserialize(&decoded_bytes).expect("Error creating genesis block");
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



impl Block {
    pub fn new(template : BlockTemplate, transactions: Vec<TxHash>) -> Self {
        Self {
            hash: template.block_hash(),
            prev_block_hash : template.prev_block_hash,
            time : template.time,
            tx_count: template.tx_count,
            nonce : template.nonce,
            merkle_root : template.merkle_root,
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
}

fn validate_block(block_hash: &BlockHash) -> bool {
    let block_hash_encoded = hex::encode(block_hash);
    block_hash_encoded.starts_with(BLOCK_DIFFICULTY)
}

impl Miner {
    pub fn new() -> Self {
        Self {
            current_nonce: 0,
            account: create_account(),
        }
    }
    pub fn mine(&mut self, prev_block_hash: &BlockHash, txs: Vec<Tx>) -> Result<Block> {
        let mut txs = txs;
        // Verify User Txs
        //Add coinbase transaction
        let tx_nonce: u128 = rand::random();
        let coinbase_tx_in = TxIn::new([0_u8; 32], 0000, self.account.pub_key);
        let coinbase_tx_out = TxOut::new(self.account.pub_key, 10);
        let coin_base_tx = Tx::signed(&self.account, tx_nonce, vec![coinbase_tx_in], vec![coinbase_tx_out])?;
        txs.insert(0, coin_base_tx);

        let mut merkle = Merkle::default();
        for tx in txs.iter() {
            let _ = merkle.update(tx.id())?;
        }
        let merkle_root = merkle.finalize().ok_or(BlockChainError::MerkleError)?;


        loop {
            let time = Utc::now().timestamp() as u32;

            let mut new_block_hash = [0_u8; 32];
            self.current_nonce = rand::random();

            let template_block = BlockTemplate::new(self.current_nonce, *prev_block_hash, time, txs.len() as u16, *merkle_root)?;
            let empty_block = [0_u8; 32];
            new_block_hash = template_block.block_hash();
            if new_block_hash != empty_block && validate_block(&new_block_hash) {
                let transactions : Vec<_>= txs.iter().map(|t| t.id().clone()).collect();
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
    use crate::blockchain::{Miner, Block, validate_block};
    use tiny_keccak::Hasher;
    use std::time::Instant;

    #[test]
    fn mine_genesis() {
        let mut miner = Miner::new();
        miner.mine()
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