use crypto::{BLOCK_HASH_LEN, HASH_LEN};
use merkle::MerkleRoot;

type BlockHash = [u8; BLOCK_HASH_LEN];
type BlockState = [u8; BLOCK_HASH_LEN];

struct Block {
    prev_hash: BlockHash,
    hash : BlockHash,
    merkle_root : MerkleRoot,
    state : BlockState
}