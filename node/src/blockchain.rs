use crypto::{BLOCK_HASH_LEN, HASH_LEN};
use merkle::MerkleRoot;

type BlockHash = [u8; BLOCK_HASH_LEN];
type BlockState = [u8; BLOCK_HASH_LEN];
type Balance = u128;
type Address = [u8; 64];
type PubKey = [u8; 32];
type Sig = [u8; 32];


struct BlockHeader {
    prev_hash: BlockHash,
    hash : BlockHash,
    merkle_root : MerkleRoot,
    state : BlockState
}

struct BlockTransaction {
    amount : Balance,
    from : Address,
    to : Address,
    pub_key : PubKey,
    sig : Sig
}

struct Block {
    header : BlockHeader,
    txs : Vec<BlockTransaction>
}

