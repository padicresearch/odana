#![feature(slice_take)]

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crypto::ecdsa::Keypair;
use crypto::SHA256;
use primitive_types::address::Address;
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

pub fn create_account_from_uri(network: Network, uri: &str) -> Account {
    let mut csprng = ChaCha20Rng::from_seed(*SHA256::digest(uri).as_fixed_bytes());
    let keypair = Keypair::generate(&mut csprng);
    let secret = H256::from(keypair.secret.to_bytes());
    let address = get_address_from_pub_key(keypair.public, network);
    Account { address, secret }
}

pub const ROOT: Address = Address([
    111, 100, 97, 110, 120, 49, 107, 122, 50, 55, 109, 112, 106, 104, 50, 113, 110, 106, 54, 56,
    112, 115, 97, 110, 54, 51, 103, 109, 116, 119, 51, 113, 52, 54, 122, 104, 97, 50, 119, 102,
    117, 99, 100, 50,
]);

#[cfg(test)]
mod tests {
    use crate::create_account_from_uri;
    use primitive_types::address::Address;
    use std::str::FromStr;
    use types::network::Network;

    #[test]
    fn test_account_from_uri() {
        let account0 = create_account_from_uri(Network::Testnet, "ama");
        let account1 = create_account_from_uri(Network::Testnet, "ama");
        assert_eq!(account0, account1);
    }

    #[test]
    fn test_address() {
        let account0 = Address::from_str("odanx1kz27mpjh2qnj68psan63gmtw3q46zha2wfucd2").unwrap();
        println!("{:?}", account0.0)
    }
}
