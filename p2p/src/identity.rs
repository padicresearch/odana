use std::str::FromStr;

use anyhow::Result;
use libp2p::bytes::{Buf, BufMut};
use libp2p::PeerId;
use prost::encoding::{DecodeContext, WireType};
use prost::DecodeError;
use serde::{Deserialize, Serialize};

use primitive_types::H256;
use types::config::NodeIdentityConfig;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct PeerNode {
    pub_key: H256,
}

impl PeerNode {
    pub fn new(pub_key: H256) -> Self {
        Self { pub_key }
    }

    pub fn pub_key(&self) -> &H256 {
        &self.pub_key
    }

    pub fn peer_id(&self) -> Result<PeerId> {
        Ok(PeerId::from_public_key(
            &libp2p::identity::PublicKey::Ed25519(libp2p::identity::ed25519::PublicKey::decode(
                self.pub_key.as_bytes(),
            )?),
        ))
    }
}

impl prost::Message for PeerNode {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::bytes::encode(1, &self.pub_key, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        match tag {
            1 => prost::encoding::bytes::merge(wire_type, &mut self.pub_key, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::bytes::encoded_len(1, &self.pub_key)
    }

    fn clear(&mut self) {
        *self = Default::default()
    }
}

#[derive(Clone, Debug)]
pub struct NodeIdentity {
    pub_key: libp2p::identity::ed25519::PublicKey,
    secret_key: libp2p::identity::ed25519::SecretKey,
    peer_id: PeerId,
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let keys = libp2p::identity::ed25519::Keypair::generate();

        let pub_key = keys.public();
        let secret_key = keys.secret();

        let peer_id =
            PeerId::from_public_key(&libp2p::identity::PublicKey::Ed25519(pub_key.clone()));

        Self {
            pub_key,
            secret_key,
            peer_id,
        }
    }

    pub fn identity_keys(&self) -> libp2p::identity::Keypair {
        libp2p::identity::Keypair::Ed25519(libp2p::identity::ed25519::Keypair::from(
            self.secret_key(),
        ))
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

    pub fn export_as_config(&self) -> NodeIdentityConfig {
        NodeIdentityConfig {
            pub_key: H256::from(self.pub_key.encode()),
            secret_key: H256::from_slice(self.secret_key.as_ref()),
            peer_id: self.peer_id.to_base58(),
        }
    }

    pub fn from_config(config: NodeIdentityConfig) -> Result<Self> {
        let mut secret_key_raw = config.secret_key.to_fixed_bytes();
        Ok(Self {
            pub_key: libp2p::identity::ed25519::PublicKey::decode(config.pub_key.as_bytes())?,
            secret_key: libp2p::identity::ed25519::SecretKey::from_bytes(&mut secret_key_raw)?,
            peer_id: PeerId::from_str(config.peer_id.as_str())?,
        })
    }

    pub fn to_p2p_node(&self) -> PeerNode {
        PeerNode::new(H256::from(self.pub_key.encode()))
    }
}

#[test]
fn test_generate_identity() {
    let identity = NodeIdentity::generate();
    println!("{:#?}", identity)
}
