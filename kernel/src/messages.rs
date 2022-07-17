use p2p::message::PeerMessage;
use types::events::LocalEventMessage;

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct KPeerMessage {
    msg: PeerMessage,
}

impl AsRef<PeerMessage> for KPeerMessage {
    fn as_ref(&self) -> &PeerMessage {
        &self.msg
    }
}


#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
pub(crate) struct KLocalMessage {
    msg: LocalEventMessage,
}

impl AsRef<LocalEventMessage> for KLocalMessage {
    fn as_ref(&self) -> &LocalEventMessage {
        &self.msg
    }
}