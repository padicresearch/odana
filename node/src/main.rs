use std::env;
use std::env::temp_dir;
use std::sync::Arc;
use std::sync::atomic::{AtomicI8, Ordering};
use std::time::SystemTime;

use clap::Parser;

use account::create_account;
use blockchain::blockchain::Tuchain;
use blockchain::column_family_names;
use consensus::barossa::{BarossaProtocol, NODE_POW_TARGET};
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::PeerList;
use p2p::start_p2p_server;
use storage::{PersistentStorage, PersistentStorageBackend};
use storage::memstore::MemStore;
use tracing::info;
use tracing::Level;
use tracing::tracing_subscriber;
use traits::Blockchain;
use types::events::LocalEventMessage;
use types::network::Network;

pub mod environment;

enum EventStream {
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
    let (node_2_peer_sender, mut node_2_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_2_node_sender, mut peer_2_node_receiver) = tokio::sync::mpsc::unbounded_channel();
    let peers = Arc::new(PeerList::new());
    let interrupt = Arc::new(AtomicI8::new(2)).clone();
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
    let kv = Arc::new(MemStore::new(column_family_names()));
    let storage = Arc::new(PersistentStorage::new(PersistentStorageBackend::InMemory(
        kv,
    )));
    let barossa_consensus = Arc::new(BarossaProtocol::new(Network::Testnet));
    let blockchain = Arc::new(
        Tuchain::initialize(
            path,
            barossa_consensus.clone(),
            storage,
            local_mpsc_sender.clone(),
        )
        .unwrap(),
    )
    .clone();

    //start_mining(blockchain.miner(), blockchain.state(), local_mpsc_sender);
    start_p2p_server(
        node_id,
        node_2_peer_receiver,
        peer_2_node_sender,
        args.peer,
        peers.clone(),
        NODE_POW_TARGET.into()
    )
    .await
    .unwrap();

    {
        let blockchain = blockchain.clone();
        let consensus = barossa_consensus.clone();
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
                    Some(EventStream::LocalMessage(msg))
                }else {
                    Some(EventStream::Unhandled)
                }
            }

            peer_msg = peer_2_node_receiver.recv() => {
                if let Some(peer) = peer_msg {
                    Some(EventStream::PeerMessage(peer))
                }else {
                    Some(EventStream::Unhandled)
                }
            }

        };

        if let Some(event) = event {
            match event {
                EventStream::PeerMessage(msg) => {
                    match msg {
                        PeerMessage::GetCurrentHead(req) => {
                            if let Ok(Some(current_head)) = blockchain.chain().current_header() {
                                node_2_peer_sender.send(PeerMessage::CurrentHead(
                                    CurrentHeadMessage::new(current_head.raw),
                                ));
                            }
                        }
                        PeerMessage::CurrentHead(msg) => {
                            println!("Received CurrentHead {:?}", msg);
                        }
                        PeerMessage::GetBlockHeader(_) => {}
                        PeerMessage::BlockHeader(_) => {}
                        PeerMessage::GetBlock(_) => {}
                        PeerMessage::Block(_) => {}
                        PeerMessage::BroadcastTransaction(tx_msg) => {
                            println!("{:?}", tx_msg)
                        }
                        PeerMessage::BroadcastBlock(block_msg) => {
                            println!("Received Block {:?}", block_msg)
                        }
                        PeerMessage::Ack(_) => {}
                        PeerMessage::ReAck(msg) => {}
                    };
                }
                EventStream::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalEventMessage::MindedBlock(block) => {
                            blockchain
                                .chain()
                                .put_chain(barossa_consensus.clone(), vec![block.clone()])
                                .unwrap();
                            interrupt.store(miner::worker::RESET, Ordering::Release);
                            node_2_peer_sender.send(PeerMessage::BroadcastBlock(
                                BroadcastBlockMessage::new(block.clone()),
                            ));
                        }
                        LocalEventMessage::BroadcastTx(tx) => {
                            node_2_peer_sender.send(PeerMessage::BroadcastTransaction(
                                BroadcastTransactionMessage::new(tx),
                            ));
                        }
                        LocalEventMessage::StateChanged { current_head } => {
                            node_2_peer_sender.send(PeerMessage::CurrentHead(
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
                                        current_head.hash(),
                                        node_current_head.hash.to_fixed_bytes(),
                                    );
                                    info!("Send message {:?}", msg);
                                }
                            }
                        }
                    }
                }
                EventStream::Unhandled => {}
            }
        }
    }
}
