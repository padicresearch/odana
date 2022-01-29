use std::env;
use std::env::temp_dir;
use std::sync::Arc;
use std::sync::atomic::AtomicI8;
use std::time::SystemTime;

use account::create_account;
use blockchain::blockchain::Tuchain;
use blockchain::column_family_names;
use blockchain::p2p::{BroadcastBlockMessage, BroadcastTransactionMessage, CurrentHeadMessage, NodeIdentity, PeerMessage, start_p2p_server};
use consensus::barossa::{BarossaProtocol, Network};
use miner::worker::start_worker;
use storage::{PersistentStorage, PersistentStorageBackend};
use storage::memstore::MemStore;
use tracing::Level;
use tracing::tracing_subscriber;
use traits::Blockchain;
use types::events::LocalEventMessage;

enum EventStream {
    LocalMessage(LocalEventMessage),
    PeerMessage(PeerMessage),
    Unhandled,
}

///tmp/tuchain
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Communications
    let (local_mpsc_sender, mut local_mpsc_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (node_2_peer_sender, mut node_2_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_2_node_sender, mut peer_2_node_receiver) = tokio::sync::mpsc::unbounded_channel();
    let interrupt = Arc::new(AtomicI8::new(2)).clone();
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("tuchain-temp–{}", time));
    // let mut tempdir = temp_dir();
    // tempdir.push("tuchain");
    let kv = Arc::new(MemStore::new(column_family_names()));
    let storage = Arc::new(PersistentStorage::new(PersistentStorageBackend::InMemory(kv)));
    let barossa_consensus = Arc::new(BarossaProtocol::new(Network::Testnet));
    let blockchain = Arc::new(Tuchain::initialize(path, barossa_consensus.clone(), storage, local_mpsc_sender.clone()).unwrap()).clone();

    {
        let blockchain = blockchain.clone();
        tokio::spawn(async move {
            let miner = create_account();
            start_worker(miner.address, local_mpsc_sender, barossa_consensus.clone(), blockchain.txpool(), blockchain.chain().state().unwrap(), blockchain.chain(), blockchain.chain().block_storage(), interrupt.clone()).unwrap();
        });
    }

    //start_mining(blockchain.miner(), blockchain.state(), local_mpsc_sender);
    start_p2p_server(
        NodeIdentity::generate(),
        node_2_peer_receiver,
        peer_2_node_sender,
    )
        .await?;

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
                    println!("Unhandled Peer Message");
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
                                    CurrentHeadMessage::new(
                                        current_head.raw
                                    ),
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
                    };
                }
                EventStream::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalEventMessage::MindedBlock(block) => {
                            println!("Minded new Block : {:?}", block);
                            node_2_peer_sender.send(PeerMessage::BroadcastBlock(
                                BroadcastBlockMessage::new(block.clone()),
                            ));
                            // match blockchain.dispatch(StateAction::AddNewBlock(block)) {
                            //     Ok(_) => {}
                            //     Err(e) => {
                            //         println!("State Dispatch Error {}", e)
                            //     }
                            // };
                        }
                        LocalEventMessage::BroadcastTx(tx) => {
                            // match blockchain.dispatch(StateAction::AddNewTransaction(tx.clone())) {
                            //     Ok(_) => {}
                            //     Err(e) => {
                            //         println!("State Dispatch Error {}", e)
                            //     }
                            // };
                            //println!("Sending Transaction to Chain : {:?}", tx);
                            node_2_peer_sender.send(PeerMessage::BroadcastTransaction(
                                BroadcastTransactionMessage::new(tx),
                            ));
                        }
                        LocalEventMessage::StateChanged {
                            current_head
                        } => {
                            node_2_peer_sender.send(PeerMessage::CurrentHead(
                                CurrentHeadMessage::new(current_head),
                            ));
                        }
                        LocalEventMessage::TxPoolPack(_) => {}
                    }
                }
                EventStream::Unhandled => {}
            }
        }
    }
}
