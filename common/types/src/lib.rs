pub mod block;
pub mod tx;
pub mod events;
pub mod account;

use serde::{Serialize, Deserialize};
use serde_big_array::big_array;
use codec::{Encoder, Decoder};
use derive_getters::Getters;

pub type BlockHash = [u8; 32];
pub type TxHash = [u8; 32];
pub type MerkleHash = [u8; 32];
pub type Sig = [u8; 64];
pub type AccountId = [u8; 32];

#[derive(Serialize, Deserialize, Getters, Debug, Clone)]
pub struct MempoolSnapsot {
    pub pending: Vec<TxHash>,
    pub valid: Vec<TxHash>,
}

impl Encoder for MempoolSnapsot {}

impl Decoder for MempoolSnapsot {}

big_array! { BigArray; }
