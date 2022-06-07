use anyhow::Result;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crypto::ecdsa::{Keypair, PublicKey, Signature};
use crypto::{RIPEMD160, SHA256};
use primitive_types::H160;
use types::account::Account;

pub const GOVERNANCE_ACCOUNTID: [u8; 32] = [
    102, 129, 71, 31, 126, 221, 234, 218, 37, 39, 104, 100, 107, 75, 80, 209, 8, 43, 33, 26, 137,
    251, 184, 15, 106, 108, 183, 54, 227, 161, 217, 70,
];

pub fn create_account() -> Account {
    let mut csprng = ChaCha20Rng::from_entropy();
    let keypair = Keypair::generate(&mut csprng);
    let pri_key = keypair.secret.to_bytes();
    let pub_key = keypair.public.to_bytes();
    let mut address = RIPEMD160::digest(SHA256::digest(&pub_key).as_bytes());
    Account { address, pri_key }
}
