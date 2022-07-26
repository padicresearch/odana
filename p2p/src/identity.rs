use std::str::FromStr;

use anyhow::Result;
use libp2p::gossipsub::Sha256Topic;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

use crypto::{generate_pow_from_pub_key, SHA256};
use primitive_types::{Compact, H192, H448, U256};
use primitive_types::{H256, U192};
use types::config::{EnvironmentConfig, NodeIdentityConfig};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq)]
pub struct PeerNode {
    pub_key: H256,
    nonce: U192,
}

impl PeerNode {
    pub fn new(pub_key: H256, nonce: U192) -> Self {
        Self { pub_key, nonce }
    }

    pub fn pow(&self) -> H256 {
        let mut pow_stamp = [0_u8; 56];
        pow_stamp[..24].copy_from_slice(&self.nonce.to_le_bytes());
        pow_stamp[24..].copy_from_slice(self.pub_key.as_bytes());
        SHA256::digest(pow_stamp)
    }

    pub fn pub_key(&self) -> &H256 {
        &self.pub_key
    }

    pub fn nonce(&self) -> &U192 {
        &self.nonce
    }

    pub fn peer_id(&self) -> Result<PeerId> {
        Ok(PeerId::from_public_key(
            &libp2p::identity::PublicKey::Ed25519(libp2p::identity::ed25519::PublicKey::decode(
                self.pub_key.as_bytes(),
            )?),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct NodeIdentity {
    pub_key: libp2p::identity::ed25519::PublicKey,
    secret_key: libp2p::identity::ed25519::SecretKey,
    peer_id: PeerId,
    nonce: U192,
}

impl NodeIdentity {
    pub fn generate(target: U256) -> Self {
        let keys = libp2p::identity::ed25519::Keypair::generate();

        let pub_key = keys.public();
        let secret_key = keys.secret();

        let peer_id =
            PeerId::from_public_key(&libp2p::identity::PublicKey::Ed25519(pub_key.clone()));

        let (nonce, _) = generate_pow_from_pub_key(H256::from(pub_key.encode()), target);

        Self {
            pub_key,
            secret_key,
            peer_id,
            nonce,
        }
    }

    pub fn identity_keys(&self) -> libp2p::identity::Keypair {
        let keys = libp2p::identity::Keypair::Ed25519(libp2p::identity::ed25519::Keypair::from(
            self.secret_key(),
        ));
        keys
    }

    pub fn secret_key(&self) -> libp2p::identity::ed25519::SecretKey {
        self.secret_key.clone()
    }

    pub fn pub_key(&self) -> &libp2p::identity::ed25519::PublicKey {
        &self.pub_key
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn nonce(&self) -> &U192 {
        &self.nonce
    }

    pub fn export_as_config(&self) -> NodeIdentityConfig {
        NodeIdentityConfig {
            pub_key: H256::from(self.pub_key.encode()),
            secret_key: H256::from_slice(self.secret_key.as_ref()),
            peer_id: self.peer_id.to_base58(),
            nonce: self.nonce,
            pow_stamp: self.stamp()
        }
    }

    pub fn from_config(config: NodeIdentityConfig) -> Result<Self> {
        let mut secret_key_raw = config.secret_key.to_fixed_bytes();
        Ok(Self {
            pub_key: libp2p::identity::ed25519::PublicKey::decode(config.pub_key.as_bytes())?,
            secret_key: libp2p::identity::ed25519::SecretKey::from_bytes(&mut secret_key_raw)?,
            peer_id: PeerId::from_str(config.peer_id.as_str())?,
            nonce: config.nonce,
        })
    }

    pub fn stamp(&self) -> H256 {
        let mut pow_stamp = [0_u8; 56];
        pow_stamp[..24].copy_from_slice(&self.nonce.to_le_bytes());
        pow_stamp[24..].copy_from_slice(&self.pub_key.encode());
        SHA256::digest(pow_stamp)
    }

    pub fn to_p2p_node(&self) -> PeerNode {
        PeerNode::new(H256::from(self.pub_key.encode()), self.nonce)
    }
}

#[test]
fn test_generate_identity() {
    let identity = NodeIdentity::generate(crypto::make_target(20.0));
    println!("{:#?}", identity)
}
