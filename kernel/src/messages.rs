use p2p::message::PeerMessage;
use types::events::LocalEventMessage;
use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct KPeerMessage {
    msg: PeerMessage,
}

impl From<PeerMessage> for KPeerMessage {
    fn from(msg: PeerMessage) -> Self {
        Self {
            msg
        }
    }
}

impl AsRef<PeerMessage> for KPeerMessage {
    fn as_ref(&self) -> &PeerMessage {
        &self.msg
    }
}


#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
pub struct KLocalMessage {
    msg: LocalEventMessage,
}

impl From<LocalEventMessage> for KLocalMessage {
    fn from(msg: LocalEventMessage) -> Self {
        Self {
            msg
        }
    }
}

impl AsRef<LocalEventMessage> for KLocalMessage {
    fn as_ref(&self) -> &LocalEventMessage {
        &self.msg
    }
}