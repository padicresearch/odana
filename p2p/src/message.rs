use codec::{Decodable, Encodable};
use primitive_types::H256;
use types::block::{Block, BlockHeader};
use types::tx::SignedTransaction;

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
        self.block_header
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("invalid message"))
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
    #[prost(message, optional, tag = "1")]
    pub block: Option<Block>,
}

impl BroadcastBlockMessage {
    pub fn new(block: Block) -> Self {
        Self { block: Some(block) }
    }

    pub fn block(&self) -> anyhow::Result<&Block> {
        self.block
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("invalid message"))
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

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct GetBlockHeaderMessage {
    #[prost(message, required, tag = "1")]
    pub from: H256,
    #[prost(message, optional, tag = "2")]
    pub to: Option<H256>,
}

impl GetBlockHeaderMessage {
    pub fn new(from: H256, to: Option<H256>) -> Self {
        Self { from, to }
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

#[derive(Eq, PartialEq, Clone, prost::Message)]
pub struct GetBlocksMessage {
    #[prost(message, repeated, tag = "1")]
    pub block_hashes: Vec<H256>,
}

impl GetBlocksMessage {
    pub fn new(block_hashes: Vec<H256>) -> Self {
        Self { block_hashes }
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
}

impl From<Msg> for PeerMessage {
    fn from(msg: Msg) -> Self {
        PeerMessage { msg: Some(msg) }
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
