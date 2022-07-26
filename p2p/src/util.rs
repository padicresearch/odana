use anyhow::Result;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};

pub fn validate_multiaddr(addr: &str) -> Result<()> {
    let addr: Multiaddr = addr.parse()?;
    let peer_id = match addr.iter().last() {
        Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
        _ => anyhow::bail!("Expect peer multiaddr to contain peer ID."),
    };
    Ok(())
}
