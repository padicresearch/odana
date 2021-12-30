use serde_big_array::big_array;

pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MerkleHash = [u8; 32];

big_array! { BigArray; }