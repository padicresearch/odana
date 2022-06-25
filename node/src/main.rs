#![feature(map_first_last)]

use std::env::temp_dir;
use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use clap::Parser;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;

use crate::environment::default_db_opts;
use account::create_account;
use blockchain::blockchain::Tuchain;
use blockchain::column_families;
use consensus::barossa::{BarossaProtocol, NODE_POW_TARGET};
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::{NetworkState, PeerList};
use p2p::request_handler::RequestHandler;
use p2p::{start_p2p_server};
use storage::{PersistentStorage, PersistentStorageBackend};
use tracing::info;
use tracing::tracing_subscriber;
use tracing::Level;
use traits::{Blockchain, ChainHeadReader, ChainReader};
use types::events::LocalEventMessage;
use types::Hash;
use types::network::Network;
use crate::downloader::Downloader;

mod downloader;
pub mod environment;

enum Event {
    LocalMessage(LocalEventMessage),
    PeerMessage(PeerMessage),
    Unhandled,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long)]
    peer: Option<String>,
    #[clap(short, long)]
    miner: bool,
}

enum NodeState {
    Idle,
    Bootstrapping,
    Synced,
}

fn broadcast_message(sender: &UnboundedSender<NodeToPeerMessage>, message: PeerMessage) -> anyhow::Result<(), SendError<NodeToPeerMessage>> {
    sender.send(NodeToPeerMessage {
        peer_id: None,
        message,
    })
}


fn send_message_to_peer(peer_id: String, sender: &UnboundedSender<NodeToPeerMessage>, message: PeerMessage) -> anyhow::Result<(), SendError<NodeToPeerMessage>> {
    sender.send(NodeToPeerMessage {
        peer_id: Some(peer_id),
        message,
    })
}

///tmp/tuchain
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    //logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Communications
    let (local_mpsc_sender, mut local_mpsc_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (node_to_peer_sender, mut node_to_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_to_node_sender, mut peer_to_node_receiver) = tokio::sync::mpsc::unbounded_channel();
    let peers = Arc::new(PeerList::new());
    let interrupt = Arc::new(AtomicI8::new(if args.miner {
        miner::worker::START
    } else {
        miner::worker::PAUSE
    }))
        .clone();
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push("tuchain");

    let node_id = NodeIdentity::generate(NODE_POW_TARGET.into());

    // let mut tempdir = temp_dir();
    // tempdir.push("tuchain");
    let database = Arc::new(rocksdb::DB::open_cf_descriptors(
        &default_db_opts(),
        path.join("context"),
        column_families(),
    )?);
    let storage = Arc::new(PersistentStorage::new(PersistentStorageBackend::RocksDB(
        database,
    )));
    let consensus = Arc::new(BarossaProtocol::new(Network::Alphanet));
    let blockchain = Arc::new(
        Tuchain::initialize(path, consensus.clone(), storage, local_mpsc_sender.clone()).unwrap(),
    )
        .clone();

    let network_state = Arc::new(NetworkState::new(peers.clone(), local_mpsc_sender.clone()));
    let handler = Arc::new(RequestHandler::new(blockchain.clone()));
    let downloader = Arc::new(Downloader::new());
    //start_mining(blockchain.miner(), blockchain.state(), local_mpsc_sender);
    start_p2p_server(
        node_id,
        node_to_peer_receiver,
        peer_to_node_sender,
        args.peer,
        peers.clone(),
        NODE_POW_TARGET.into(),
        network_state.clone(),
        blockchain.chain(),
        handler
    )
        .await
    .unwrap();

    {
        let blockchain = blockchain.clone();
        let consensus = consensus.clone();
        let interrupt = interrupt.clone();
        tokio::spawn(async move {
            let miner = create_account();
            start_worker(
                miner.address,
                local_mpsc_sender,
                consensus,
                blockchain.txpool(),
                blockchain.chain(),
                blockchain.chain().block_storage(),
                interrupt,
            )
                .unwrap();
        });
    }

    {
        let blockchain = blockchain.clone();
        let consensus = consensus.clone();
        let interrupt = interrupt.clone();
        let downloader = downloader.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(Some(node_current_head)) = blockchain.chain().current_header()
                {
                    match blockchain.chain().block_storage().get_block_by_level(node_current_head.raw.level + 1) {
                        Ok(Some(block)) => {
                            blockchain
                                .chain()
                                .put_chain(consensus.clone(), vec![block])
                                .unwrap();
                        }
                        _ => {
                            if downloader.is_downloading() {
                                interrupt.store(miner::worker::PAUSE, Ordering::Release);
                            } else if !downloader.is_downloading() && args.miner {
                                if interrupt.load(Ordering::Acquire) == miner::worker::PAUSE {
                                    interrupt.store(miner::worker::RESET, Ordering::Release);
                                }
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(10))
                }
            }
        });
    }


    loop {
        let event = tokio::select! {
            local_msg = local_mpsc_receiver.recv() => {
                if let Some(msg) = local_msg {
                    //println!("Local Message : {:?}", msg);
                    Some(Event::LocalMessage(msg))
                }else {
                    Some(Event::Unhandled)
                }
            }

            peer_msg = peer_to_node_receiver.recv() => {
                if let Some(peer) = peer_msg {
                    Some(Event::PeerMessage(peer))
                }else {
                    Some(Event::Unhandled)
                }
            }

        };

        if let Some(event) = event {
            match event {
                Event::PeerMessage(msg) => {
                    match msg {
                        PeerMessage::CurrentHead(msg) => {
                        }
                        PeerMessage::BlockHeader(msg) => {
                            info!(count = ?msg.block_headers.len(), "Imported headers");
                            downloader.enqueue(msg.block_headers);
                            let next_blocks = downloader.next_blocks_to_download();
                            match network_state.highest_peer() {
                                None => {}
                                Some(peer_id) => {
                                    send_message_to_peer(peer_id.clone(), &node_to_peer_sender, PeerMessage::GetBlocks(BlocksToDownloadMessage::new(next_blocks)));
                                    match downloader.last_header_in_queue() {
                                        None => {}
                                        Some(from) => {
                                            send_message_to_peer(peer_id, &node_to_peer_sender, PeerMessage::GetBlockHeader(GetBlockHeaderMessage::new(from, None)));
                                        }
                                    }
                                }
                            }
                        }
                        PeerMessage::Blocks(msg) => {
                            // TODO: Verify Blocks
                            // TODO: Store Blocks

                            for block in msg.blocks.iter() {
                                blockchain.chain().block_storage().put(block.clone())?;
                                downloader.finish_download(&block.hash());
                            }

                            let next_blocks = downloader.next_blocks_to_download();
                            match network_state.highest_peer() {
                                None => {}
                                Some(peer_id) => {
                                    send_message_to_peer(peer_id, &node_to_peer_sender, PeerMessage::GetBlocks(BlocksToDownloadMessage::new(next_blocks)));
                                }
                            }
                        }
                        PeerMessage::BroadcastTransaction(msg) => {
                            let txpool = blockchain.txpool();
                            let mut txpool = txpool.write().unwrap();
                            txpool.add_remote(msg.tx).unwrap()
                        }
                        PeerMessage::BroadcastBlock(msg) => {
                            let block = msg.block;
                            // TODO: validate block
                            // TODO: Check if future block is not further than 3 days
                            blockchain.chain().block_storage().put(block.clone())?;
                        }
                        _ => {}
                    };
                }
                Event::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalEventMessage::MindedBlock(block) => {
                            blockchain.chain().block_storage().put(block.clone())?;
                            interrupt.store(miner::worker::RESET, Ordering::Release);
                            broadcast_message(&node_to_peer_sender, PeerMessage::BroadcastBlock(
                                BroadcastBlockMessage::new(block.clone()),
                            ));
                        }
                        LocalEventMessage::BroadcastTx(tx) => {
                            broadcast_message(&node_to_peer_sender, PeerMessage::BroadcastTransaction(
                                BroadcastTransactionMessage::new(tx),
                            ));
                        }
                        LocalEventMessage::StateChanged { current_head } => {
                            broadcast_message(&node_to_peer_sender, PeerMessage::CurrentHead(CurrentHeadMessage::new(current_head)));
                        }
                        LocalEventMessage::TxPoolPack(_) => {}
                        LocalEventMessage::NetworkHighestHeadChanged {
                            peer_id,
                            current_head,
                        } => {
                            if let Ok(Some(node_current_head)) = blockchain.chain().current_header()
                            {
                                if node_current_head.raw.level < current_head.level {
                                    // Start downloading blocks from the Peer
                                }
                            }
                        }
                        LocalEventMessage::NetworkNewPeerConnection { stats, peer_id } => {
                            info!(pending = ?stats.0, connected = ?stats.1, "Peer connection");
                            // Send get headers to peer
                            if let Ok(Some(node_current_head)) = blockchain.chain().current_header()
                            {
                                let msg = GetBlockHeaderMessage::new(
                                    node_current_head.hash.0,
                                    None,
                                );
                                send_message_to_peer(peer_id, &node_to_peer_sender, PeerMessage::GetBlockHeader(msg)).unwrap();
                            }
                        }
                    }
                }
                Event::Unhandled => {}
            }
        }
    }
}
