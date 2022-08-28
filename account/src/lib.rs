#![feature(slice_take)]

use crypto::{keccak256};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use bech32::{ToBase32, Variant};
use crypto::ecdsa::{Keypair, PublicKey};
use primitive_types::{H160, H256};
use types::account::{Account, Address42, get_address_from_pub_key};
use types::network::Network;

pub fn create_account(network : Network) -> Account {
    let mut csprng = ChaCha20Rng::from_entropy();
    let keypair = Keypair::generate(&mut csprng);
    let secret = H256::from(keypair.secret.to_bytes());
    let address = get_address_from_pub_key(keypair.public, network);
    Account { address, secret }
}
