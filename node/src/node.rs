use crate::environment::default_db_opts;
use crate::sync::{SyncMode, SyncService};
use crate::{Args, RunArgs};
use account::create_account;
use anyhow::Result;
use blockchain::blockchain::Tuchain;
use blockchain::column_families;
use clap::Parser;
use consensus::barossa::{BarossaProtocol, NODE_POW_TARGET};
use directories::UserDirs;
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::{NetworkState, PeerList};
use p2p::request_handler::RequestHandler;
use p2p::start_p2p_server;
use primitive_types::H256;
use std::env::temp_dir;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use storage::{PersistentStorage, PersistentStorageBackend};
use temp_dir::TempDir;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;
use rpc::start_rpc_server;
use tracing::tracing_subscriber;
use tracing::Level;
use tracing::{error, info};
use traits::{Blockchain, ChainHeadReader, ChainReader, Handler};
use types::block::Block;
use types::config::{EnvironmentConfig, NodeIdentityConfig};
use types::events::LocalEventMessage;
use types::network::Network;
use types::Hash;

enum Event {
    LocalMessage(LocalEventMessage),
    PeerMessage(PeerMessage),
    Unhandled,
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

pub(crate) fn run(args: &RunArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { _start_node(args).await })
}

async fn _start_node(args: &RunArgs) -> Result<()> {
    // Config

    let user_dir = UserDirs::new().unwrap();
    let mut default_datadir = PathBuf::from(user_dir.home_dir());
    default_datadir.push("tuchain");
    let mut datadir = args.datadir.clone().unwrap_or(default_datadir);

    let mut config = EnvironmentConfig::default();

    if let Some(config_file_path) = &args.config_file {
        let config_file = OpenOptions::new()
            .read(true)
            .open(config_file_path.as_path())?;
        config = serde_json::from_reader(config_file)?;
    } else {
        let res: Result<EnvironmentConfig, _> = OpenOptions::new()
            .read(true)
            .open(datadir.join("config.json"))
            .map_err(|e| anyhow::anyhow!("{}", e))
            .and_then(|config_file| {
                serde_json::from_reader(config_file).map_err(|e| anyhow::anyhow!("{}", e))
            });

        match res {
            Ok(c) => {
                config = c;
            }
            Err(_) => {}
        }
    }

    if let Some(network) = args.network {
        config.network = network;
    }

    config.peers = args.peer.clone();

    if let Some(network) = args.network {
        config.network = network;
    }

    if let Some(coinbase) = args.miner {
        config.miner = Some(coinbase)
    }

    if let Some(expected_pow) = args.expected_pow {
        config.expected_pow = expected_pow
    }

    if let Some(host) = &args.host {
        config.host = host.clone()
    }

    if let Some(p2p_port) = args.p2p_port {
        config.p2p_port = p2p_port
    }

    if let Some(rpc_port) = args.rpc_port {
        config.rpc_port = rpc_port
    }

    if let Some(identity_file) = &args.identity_file {
        config.identity_file = Some(identity_file.clone())
    }

    let env = Arc::new(config);
    // Communications
    let (local_mpsc_sender, mut local_mpsc_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (node_to_peer_sender, mut node_to_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_to_node_sender, mut peer_to_node_receiver) = tokio::sync::mpsc::unbounded_channel();
    let node_to_peer_sender = Arc::new(node_to_peer_sender);
    let interrupt = Arc::new(AtomicI8::new(miner::worker::START));

    // TODO; Refactor [Directory]

    let node_id = NodeIdentity::from_config(
        NodeIdentityConfig::open(
            env.identity_file
                .clone()
                .unwrap_or(datadir.join("identity.json")),
        )
            .expect("identity file not found"),
    )
        .expect("failed to read identity file");

    let database = Arc::new(rocksdb::DB::open_cf_descriptors(
        &default_db_opts(),
        datadir.join("context"),
        column_families(),
    )?);
    let storage = Arc::new(PersistentStorage::new(PersistentStorageBackend::RocksDB(
        database,
    )));
    let consensus = Arc::new(BarossaProtocol::new(env.network));
    let blockchain = Arc::new(
        Tuchain::initialize(
            datadir,
            consensus.clone(),
            storage,
            local_mpsc_sender.clone(),
        )
            .unwrap(),
    )
        .clone();

    let network_state = Arc::new(NetworkState::new(local_mpsc_sender.clone()));
    let handler = Arc::new(RequestHandler::new(
        blockchain.clone(),
        network_state.clone(),
    ));

    let mut sync_service = {
        let blockchain = blockchain.clone();
        let consensus = consensus.clone();
        //let system = System::new();
        SyncService::new(
            blockchain.chain(),
            blockchain.txpool(),
            node_to_peer_sender.clone(),
            consensus,
            blockchain.chain().block_storage(),
            Arc::new(SyncMode::Normal),
        )
    };

    let identity_expected_pow = crypto::make_target(env.expected_pow);

    start_p2p_server(
        env.clone(),
        node_id,
        node_to_peer_receiver,
        peer_to_node_sender,
        args.peer.clone(),
        identity_expected_pow,
        network_state.clone(),
        blockchain.chain(),
        handler,
    )
        .await?;

    {
        let blockchain = blockchain.clone();
        let env = env.clone();
        tokio::spawn(start_rpc_server(blockchain.chain().clone(), env));
    }


    if let Some(miner) = env.miner {
        let blockchain = blockchain.clone();
        let consensus = consensus.clone();
        let interrupt = interrupt.clone();
        let network_state = network_state.clone();
        tokio::spawn(async move {
            start_worker(
                miner,
                local_mpsc_sender,
                consensus,
                blockchain.txpool(),
                blockchain.chain(),
                network_state,
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
                            let block = msg.block;
                            // TODO: validate block
                            // TODO: Check if future block is not further than 3 days
                            blockchain.chain().block_storage().put(block.clone())?;
                        }
                        msg => {
                            sync_service.handle(msg);
                        }
                    };
                }
                Event::LocalMessage(local_msg) => match local_msg {
                    LocalEventMessage::MindedBlock(block) => {
                        broadcast_message(
                            &node_to_peer_sender,
                            PeerMessage::BroadcastBlock(BroadcastBlockMessage::new(block.clone())),
                        )
                            .unwrap();
                    }
                    LocalEventMessage::BroadcastTx(tx) => {
                        broadcast_message(
                            &node_to_peer_sender,
                            PeerMessage::BroadcastTransaction(BroadcastTransactionMessage::new(tx)),
                        )
                            .unwrap();
                    }
                    LocalEventMessage::StateChanged { current_head } => {
                        broadcast_message(
                            &node_to_peer_sender,
                            PeerMessage::CurrentHead(CurrentHeadMessage::new(current_head)),
                        )
                            .unwrap();
                    }
                    LocalEventMessage::NetworkNewPeerConnection { stats, peer_id } => {
                        info!(pending = ?stats.0, connected = ?stats.1, "Peer connection");
                    }
                    msg => {
                        sync_service.handle(msg);
                    }
                },
                Event::Unhandled => {}
            }
        }
    }
}
