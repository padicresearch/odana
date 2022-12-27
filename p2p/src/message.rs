use libp2p::bytes::{Buf, BufMut};
use prost::encoding::{BytesAdapter, DecodeContext, WireType};
use prost::DecodeError;

use codec::{Decodable, Encodable};
use primitive_types::H256;
use types::block::{Block, BlockHeader};
use types::tx::SignedTransaction;

use crate::identity::PeerNode;

#[derive(Eq, PartialEq, prost::Message, Clone)]
pub struct CurrentHeadMessage {
    #[prost(message, tag = "1")]
    pub block_header: Option<BlockHeader>,
}

impl CurrentHeadMessage {
    pub fn new(block_header: BlockHeader) -> Self {
        Self {
            block_header: Some(block_header),
        }
    }

    pub fn block_header(&self) -> anyhow::Result<&BlockHeader> {
        self.block_header.as_ref().ok_or_else(||anyhow::anyhow!("invalid message"))
    }
}

#[derive(prost::Message, Clone, Eq, PartialEq)]
pub struct BroadcastTransactionMessage {
    #[prost(message, repeated, tag = "1")]
    pub tx: Vec<SignedTransaction>,
}

impl BroadcastTransactionMessage {
    pub fn new(tx: Vec<SignedTransaction>) -> Self {
        Self { tx }
    }
}

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct BroadcastBlockMessage {
    #[prost(message, tag = "1")]
    pub block: Option<Block>,
}

impl BroadcastBlockMessage {
    pub fn new(block: Block) -> Self {
        Self { block: Some(block) }
    }

    pub fn block(&self) -> anyhow::Result<&Block> {
        self.block.as_ref().ok_or_else(||anyhow::anyhow!("invalid message"))
    }
}

#[derive(prost::Message, Eq, PartialEq, Clone)]
pub struct GetCurrentHeadMessage {
    #[prost(string, tag = "1")]
    pub sender: String,
}

impl GetCurrentHeadMessage {
    pub fn new(sender: String) -> Self {
        Self { sender }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct GetBlockHeaderMessage {
    pub from: H256,
    pub to: Option<H256>,
}

impl GetBlockHeaderMessage {
    pub fn new(from: H256, to: Option<H256>) -> Self {
        Self { from, to }
    }
}

impl prost::Message for GetBlockHeaderMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::bytes::encode(1, &self.from, buf);
        if let Some(to) = &self.to {
            prost::encoding::bytes::encode(2, to, buf)
        }
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        match tag {
            1 => prost::encoding::bytes::merge(wire_type, &mut self.from, buf, ctx),
            2 => {
                let value = &mut self.to;
                prost::encoding::bytes::merge(
                    wire_type,
                    value.get_or_insert_with(Default::default),
                    buf,
                    ctx,
                )
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        (if !self.from.is_empty() {
            prost::encoding::bytes::encoded_len(1u32, &self.from)
        } else {
            0
        }) + self
            .to
            .as_ref()
            .map_or(0, |value| prost::encoding::bytes::encoded_len(2u32, value))
    }

    fn clear(&mut self) {
        self.from = Default::default();
        self.to = None
    }
}

#[derive(prost::Message, Eq, PartialEq, Clone)]
pub struct FindBlocksMessage {
    #[prost(uint32, tag = "1")]
    pub from: u32,
    #[prost(uint32, tag = "2")]
    pub limit: u32,
}

impl FindBlocksMessage {
    pub fn new(from: u32, limit: u32) -> Self {
        Self { from, limit }
    }
}

#[derive(prost::Message, Eq, PartialEq, Clone)]
pub struct BlockTransactionsMessage {
    #[prost(message, repeated, tag = "1")]
    pub txs: Vec<SignedTransaction>,
}

impl BlockTransactionsMessage {
    pub fn new(txs: Vec<SignedTransaction>) -> Self {
        Self { txs }
    }
}

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct BlocksMessage {
    #[prost(message, repeated, tag = "1")]
    pub blocks: Vec<Block>,
}

impl BlocksMessage {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct BlockHeaderMessage {
    #[prost(message, repeated, tag = "1")]
    pub block_headers: Vec<BlockHeader>,
}

impl BlockHeaderMessage {
    pub fn new(block_headers: Vec<BlockHeader>) -> Self {
        Self { block_headers }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct GetBlocksMessage {
    pub block_hashes: Vec<H256>,
}

impl GetBlocksMessage {
    pub fn new(block_hashes: Vec<H256>) -> Self {
        Self { block_hashes }
    }
}

impl prost::Message for GetBlocksMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        prost::encoding::bytes::encode_repeated(1, &self.block_hashes, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        if tag == 1 {
            prost::encoding::bytes::merge_repeated(wire_type, &mut self.block_hashes, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        prost::encoding::bytes::encoded_len_repeated(1, &self.block_hashes)
    }

    fn clear(&mut self) {
        self.block_hashes.clear()
    }
}

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct ReAckMessage {
    #[prost(message, tag = "1")]
    pub node_info: Option<PeerNode>,
    #[prost(message, tag = "2")]
    pub current_header: Option<BlockHeader>,
}

impl ReAckMessage {
    pub fn new(node_info: PeerNode, current_header: BlockHeader) -> Self {
        Self {
            node_info: Some(node_info),
            current_header: Some(current_header),
        }
    }

    pub fn node_info(&self) -> anyhow::Result<PeerNode> {
        self.node_info.ok_or_else(||anyhow::anyhow!("invalid message"))
    }

    pub fn current_header(&self) -> anyhow::Result<BlockHeader> {
        self.current_header.ok_or_else(||anyhow::anyhow!("invalid message"))
    }
}

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct AckMessage;

impl AckMessage {
    pub(crate) fn new() -> AckMessage {
        AckMessage
    }
}

#[derive(prost::Message, Eq, PartialEq, Clone)]
pub struct AdvertiseMessage {
    #[prost(string, repeated, tag = "1")]
    pub peers: Vec<String>,
}

impl AdvertiseMessage {
    pub fn new(peers: Vec<String>) -> Self {
        Self { peers }
    }
}

#[derive(Clone, PartialEq, Eq, prost::Message)]
pub struct PeerMessage {
    #[prost(oneof = "Msg", tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11")]
    pub msg: Option<Msg>,
}

impl PeerMessage {
    pub fn new(msg: Msg) -> Self {
        Self { msg: Some(msg) }
    }
}

#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum Msg {
    #[prost(message, tag = "1")]
    GetCurrentHead(CurrentHeadMessage),
    #[prost(message, tag = "2")]
    CurrentHead(CurrentHeadMessage),
    #[prost(message, tag = "3")]
    GetBlockHeader(GetBlockHeaderMessage),
    #[prost(message, tag = "4")]
    GetBlocks(GetBlocksMessage),
    #[prost(message, tag = "5")]
    FindBlocks(FindBlocksMessage),
    #[prost(message, tag = "6")]
    BlockHeader(BlockHeaderMessage),
    #[prost(message, tag = "7")]
    Blocks(BlocksMessage),
    #[prost(message, tag = "8")]
    BroadcastTransaction(BroadcastTransactionMessage),
    #[prost(message, tag = "9")]
    BroadcastBlock(BroadcastBlockMessage),
    #[prost(message, tag = "10")]
    Ack(AckMessage),
    #[prost(message, tag = "11")]
    ReAck(ReAckMessage),
}

impl From<Msg> for PeerMessage {
    fn from(msg: Msg) -> Self {
        PeerMessage {
            msg: Some(msg)
        }
    }
}
#[derive(Debug)]
pub struct NodeToPeerMessage {
    pub peer_id: Option<String>,
    pub message: Msg,
}

impl Encodable for PeerMessage {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(prost::Message::encode_to_vec(self))
    }
}

impl Decodable for PeerMessage {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        prost::Message::decode(buf).map_err(|e| e.into())
    }
}
