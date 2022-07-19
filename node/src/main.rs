#![feature(map_first_last)]

use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use actix::{Actor, System};

use clap::Parser;
use temp_dir::TempDir;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;

use crate::downloader::Downloader;
use crate::environment::default_db_opts;
use account::create_account;
use blockchain::blockchain::Tuchain;
use blockchain::column_families;
use consensus::barossa::{BarossaProtocol, NODE_POW_TARGET};
use kernel::{SyncManager, SyncMode};
use kernel::messages::{KLocalMessage, KPeerMessage};
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::{NetworkState, PeerList};
use p2p::request_handler::RequestHandler;
use p2p::start_p2p_server;
use primitive_types::H256;
use storage::{PersistentStorage, PersistentStorageBackend};
use tracing::tracing_subscriber;
use tracing::Level;
use tracing::{error, info};
use traits::{Blockchain, ChainHeadReader, ChainReader};
use types::block::Block;
use types::events::LocalEventMessage;
use types::network::Network;
use types::Hash;

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
    datadir: Option<PathBuf>,
    #[clap(short, long)]
    miner: bool,
}

enum NodeState {
    Idle,
    Bootstrapping,
    Synced,
}

fn broadcast_message(
    sender: &UnboundedSender<NodeToPeerMessage>,
    message: PeerMessage,
) -> anyhow::Result<(), SendError<NodeToPeerMessage>> {
    sender.send(NodeToPeerMessage {
        peer_id: None,
        message,
    })
}

fn send_message_to_peer(
    peer_id: String,
    sender: &UnboundedSender<NodeToPeerMessage>,
    message: PeerMessage,
) -> anyhow::Result<(), SendError<NodeToPeerMessage>> {
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
    let mut path = PathBuf::from("/tmp");
    path.push("tuchain");

    println!("datadir {:?}", path);

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
    let consensus = Arc::new(BarossaProtocol::new(Network::Testnet));
    let blockchain = Arc::new(
        Tuchain::initialize(path, consensus.clone(), storage, local_mpsc_sender.clone()).unwrap(),
    )
        .clone();

    let network_state = Arc::new(NetworkState::new(peers.clone(), local_mpsc_sender.clone()));
    let handler = Arc::new(RequestHandler::new(blockchain.clone(), network_state.clone()));
    let downloader = Arc::new(Downloader::new());

    let mut sync_service = {
        let blockchain = blockchain.clone();
        let block_storage = blockchain.chain().block_storage();
        let consensus = consensus.clone();
        //let system = System::new();
        SyncManager::new(blockchain.chain(), consensus, block_storage, Arc::new(SyncMode::Normal))
    };

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
        handler,
    )
        .await
    .unwrap();

    {
        let blockchain = blockchain.clone();
        let block_storage = blockchain.chain().block_storage();
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
                block_storage,
                blockchain.chain().block_storage(),
                interrupt,
            )
                .unwrap();
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
                        PeerMessage::BroadcastTransaction(msg) => {
                            let txpool = blockchain.txpool();
                            let mut txpool = txpool.write().unwrap();
                            txpool.add_remote(msg.tx).unwrap()
                        }
                        PeerMessage::BroadcastBlock(msg) => {
                            println!("Got BroadcastBlock");
                            let block = msg.block;
                            // TODO: validate block
                            // TODO: Check if future block is not further than 3 days
                            blockchain.chain().block_storage().put(block.clone())?;
                        }
                        msg => {
                            println!("Sending peer message to sync service");
                            sync_service.handle_peer(msg);
                        }
                    };
                }
                Event::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalEventMessage::MindedBlock(block) => {
                            broadcast_message(
                                &node_to_peer_sender,
                                PeerMessage::BroadcastBlock(BroadcastBlockMessage::new(
                                    block.clone(),
                                )),
                            ).unwrap();
                        }
                        LocalEventMessage::BroadcastTx(tx) => {
                            broadcast_message(
                                &node_to_peer_sender,
                                PeerMessage::BroadcastTransaction(
                                    BroadcastTransactionMessage::new(tx),
                                ),
                            )
                                .unwrap();
                        }
                        LocalEventMessage::StateChanged { current_head } => {
                            broadcast_message(
                                &node_to_peer_sender,
                                PeerMessage::CurrentHead(CurrentHeadMessage::new(current_head)),
                            ).unwrap();
                        }
                        LocalEventMessage::NetworkNewPeerConnection { stats, peer_id } => {
                            info!(pending = ?stats.0, connected = ?stats.1, "Peer connection");
                        }
                        msg => {
                            println!("Sending local message to sync service");
                            sync_service.handle_local(msg);
                        }
                    }
                }
                Event::Unhandled => {}
            }
        }
    }
}
