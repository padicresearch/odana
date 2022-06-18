use std::env::temp_dir;
use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use clap::Parser;

use crate::environment::default_db_opts;
use account::create_account;
use blockchain::blockchain::Tuchain;
use blockchain::{column_families, column_family_names};
use consensus::barossa::{BarossaProtocol, NODE_POW_TARGET};
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::{NetworkState, PeerList};
use p2p::start_p2p_server;
use storage::memstore::MemStore;
use storage::{PersistentStorage, PersistentStorageBackend};
use tracing::info;
use tracing::tracing_subscriber;
use tracing::Level;
use traits::{Blockchain, ChainHeadReader, ChainReader};
use types::events::LocalEventMessage;
use types::network::Network;

mod download_manager;
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
    println!("{:#?}", node_id);

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
                        PeerMessage::GetCurrentHead(req) => {
                            if let Ok(Some(current_head)) = blockchain.chain().current_header() {
                                node_to_peer_sender.send(PeerMessage::CurrentHead(
                                    CurrentHeadMessage::new(current_head.raw),
                                ));
                            }
                        }
                        PeerMessage::CurrentHead(msg) => {
                            println!("Received CurrentHead {:?}", msg);
                            println!("Network State {:?}", msg);
                        }
                        PeerMessage::GetBlockHeader(msg) => {
                            println!("Received GetBlockHeader {:?}", msg);
                            let mut headers = Vec::with_capacity(2000);
                            let res = blockchain
                                .chain()
                                .block_storage()
                                .get_block_by_hash(&msg.from);
                            let mut level = match res {
                                Ok(Some(block)) => block.level(),
                                _ => -1,
                            };
                            loop {
                                let res = blockchain
                                    .chain()
                                    .block_storage()
                                    .get_header_by_level(level);
                                let header = match res {
                                    Ok(Some(block)) => block.raw,
                                    _ => break,
                                };

                                if headers.len() >= 2000 {
                                    break;
                                }

                                if Some(header.hash()) == msg.to {
                                    headers.push(header);
                                    break;
                                }
                                headers.push(header);
                                level += 1;
                            }

                            node_to_peer_sender
                                .send(PeerMessage::BlockHeader(BlockHeaderMessage::new(headers)));
                        }
                        PeerMessage::BlockHeader(msg) => {
                            println!("{:#?}", msg.block_headers);
                        }
                        PeerMessage::GetBlocks(msg) => {
                            let mut blocks = Vec::with_capacity(msg.block_hashes.len());
                            for hash in msg.block_hashes.iter() {
                                let res =
                                    blockchain.chain().block_storage().get_block_by_hash(hash);
                                match res {
                                    Ok(Some(block)) => blocks.push(block),
                                    _ => break,
                                }
                            }

                            if blocks.len() != msg.block_hashes.len() {
                                blocks.clear();
                            } else {
                                node_to_peer_sender
                                    .send(PeerMessage::Blocks(BlocksMessage::new(blocks)));
                            }
                        }
                        PeerMessage::Blocks(msg) => {
                            // TODO: Verify Blocks
                            // TODO: Store Blocks
                            blockchain.chain().put_chain(consensus.clone(), msg.blocks);
                        }
                        PeerMessage::BroadcastTransaction(msg) => {
                            println!("{:?}", msg.tx)
                        }
                        PeerMessage::BroadcastBlock(msg) => {
                            println!("Received Block {:?}", msg)
                        }
                        _ => {}
                    };
                }
                Event::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalEventMessage::MindedBlock(block) => {
                            blockchain.chain().block_storage().put(block.clone())?;
                            blockchain
                                .chain()
                                .put_chain(consensus.clone(), vec![block.clone()])
                                .unwrap();
                            interrupt.store(miner::worker::RESET, Ordering::Release);
                            node_to_peer_sender.send(PeerMessage::BroadcastBlock(
                                BroadcastBlockMessage::new(block.clone()),
                            ));
                        }
                        LocalEventMessage::BroadcastTx(tx) => {
                            node_to_peer_sender.send(PeerMessage::BroadcastTransaction(
                                BroadcastTransactionMessage::new(tx),
                            ));
                        }
                        LocalEventMessage::StateChanged { current_head } => {
                            node_to_peer_sender.send(PeerMessage::CurrentHead(
                                CurrentHeadMessage::new(current_head),
                            ));
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
                                    let msg = GetBlockHeaderMessage::new(
                                        node_current_head.hash.0,
                                        None,
                                    );
                                    info!("Send message block download {:?}", msg);
                                }
                            }
                        }
                        LocalEventMessage::NetworkNewPeerConnection { stats } => {
                            info!(pending = ?stats.0, connected = ?stats.1, "Peer connection");
                        }
                    }
                }
                Event::Unhandled => {}
            }
        }
    }
}
