#![feature(slice_take)]

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crypto::ecdsa::Keypair;
use primitive_types::H256;
use types::account::{get_address_from_pub_key, Account};
use types::network::Network;

pub fn create_account(network: Network) -> Account {
    let mut csprng = ChaCha20Rng::from_entropy();
    let keypair = Keypair::generate(&mut csprng);
    let secret = H256::from(keypair.secret.to_bytes());
    let address = get_address_from_pub_key(keypair.public, network);
    Account { address, secret }
}
