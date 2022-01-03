pub mod block;
pub mod tx;

use serde_big_array::big_array;

pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MerkleHash = [u8; 32];
pub type Sig = [u8; 64];
pub type AccountId = [u8; 32];

big_array! { BigArray; }
