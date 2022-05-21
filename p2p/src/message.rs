use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

use codec::{impl_codec, Decoder, Encoder};
use types::block::{Block, BlockHeader};
use types::tx::Transaction;
use types::Hash;

use crate::identity::PeerNode;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct CurrentHeadMessage {
    pub block_header: BlockHeader,
}

impl CurrentHeadMessage {
    pub fn new(block_header: BlockHeader) -> Self {
        Self { block_header }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BroadcastTransactionMessage {
    tx: Transaction,
}

impl BroadcastTransactionMessage {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    pub fn tx(self) -> Transaction {
        self.tx
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BroadcastBlockMessage {
    block: Block,
}

impl BroadcastBlockMessage {
    pub fn new(block: Block) -> Self {
        Self { block }
    }

    pub fn block(self) -> Block {
        self.block
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetCurrentHeadMessage {
    pub sender: String,
}

impl GetCurrentHeadMessage {
    pub fn new(sender: String) -> Self {
        Self { sender }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetBlockHeaderMessage {
    pub from: Hash,
    pub to: Hash,
}

impl GetBlockHeaderMessage {
    pub fn new(from: Hash, to: Hash) -> Self {
        Self { from, to }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BlockTransactionsMessage {
    pub txs: Vec<Transaction>,
}

impl BlockTransactionsMessage {
    pub fn new(txs: Vec<Transaction>) -> Self {
        Self { txs }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct BlockHeaderMessage {
    pub block_headers: Vec<BlockHeader>,
}

impl BlockHeaderMessage {
    pub fn new(block_headers: Vec<BlockHeader>) -> Self {
        Self { block_headers }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct GetBlockTransactionsMessage {
    pub sender: String,
    pub tx_ids: Vec<Hash>,
}

impl GetBlockTransactionsMessage {
    pub fn new(sender: PeerId, tx_ids: Vec<Hash>) -> Self {
        Self {
            sender: sender.to_string(),
            tx_ids,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ReAckMessage {
    pub node_info: PeerNode,
    pub current_header: BlockHeader,
}

impl ReAckMessage {
    pub fn new(node_info: PeerNode, current_header: BlockHeader) -> Self {
        Self {
            node_info,
            current_header,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct AdvertiseMessage {
    pub peers: Vec<String>,
}

impl AdvertiseMessage {
    pub fn new(peers: Vec<String>) -> Self {
        Self { peers }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum PeerMessage {
    GetCurrentHead(GetCurrentHeadMessage),
    CurrentHead(CurrentHeadMessage),
    GetBlockHeader(GetBlockHeaderMessage),
    BlockHeader(BlockHeaderMessage),
    GetBlock(Block),
    Block(Block),
    BroadcastTransaction(BroadcastTransactionMessage),
    BroadcastBlock(BroadcastBlockMessage),
    Ack(Multiaddr),
    ReAck(ReAckMessage),
}

impl_codec!(PeerMessage);
