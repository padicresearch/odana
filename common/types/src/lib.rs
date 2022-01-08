pub mod account;
pub mod block;
pub mod events;
pub mod tx;

use codec::{Decoder, Encoder};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
pub type Hash = [u8; 32];
pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MerkleHash = [u8; 32];
pub type Sig = [u8; 64];
pub type PubKey = [u8; 32];

#[derive(Serialize, Deserialize, Getters, Debug, Clone)]
pub struct MempoolSnapsot {
    pub pending: Vec<TxHash>,
    pub valid: Vec<TxHash>,
}

impl Encoder for MempoolSnapsot {}

impl Decoder for MempoolSnapsot {}

big_array! { BigArray; }
