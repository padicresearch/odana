use std::fs::{File, OpenOptions};
use std::sync::atomic::AtomicI8;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;

use blockchain::blockchain::Chain;
use blockchain::column_families;
use consensus::barossa::BarossaProtocol;
use miner::worker::start_worker;
use p2p::identity::NodeIdentity;
use p2p::message::*;
use p2p::peer_manager::NetworkState;
use p2p::request_handler::RequestHandler;
use p2p::start_p2p_server;
use rpc::start_rpc_server;
use storage::{PersistentStorage, PersistentStorageBackend};
use tracing::tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing::{info, tracing_subscriber, warn};
use traits::Handler;
use types::config::{EnvironmentConfig, NodeIdentityConfig};
use types::events::LocalEventMessage;

use crate::environment::default_db_opts;
use crate::sync::{SyncMode, SyncService};
use crate::{Level, RunArgs};

enum Event {
    LocalMessage(LocalEventMessage),
    PeerMessage(Msg),
    Unhandled,
}

fn broadcast_message(
    sender: &UnboundedSender<NodeToPeerMessage>,
    message: Msg,
) -> anyhow::Result<(), SendError<NodeToPeerMessage>> {
    sender.send(NodeToPeerMessage {
        peer_id: None,
        message,
    })
}

pub(crate) fn run(args: &RunArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { _start_node(args).await })
}

async fn _start_node(args: &RunArgs) -> Result<()> {
    let env = setup_environment(args)?;
    // Setup Logging
    let log_level: Level = args.log_level.into();
    let debug_log = Arc::new(File::create(env.datadir.join("debug.log"))?);

    let mk_writer = std::io::stderr.with_max_level(Level::ERROR).or_else(
        std::io::stdout
            .with_max_level(log_level.into())
            .and(debug_log.with_max_level(Level::DEBUG)),
    );

    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_writer(mk_writer)
        .init();

    // Communications
    let (local_mpsc_sender, mut local_mpsc_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (node_to_peer_sender, node_to_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_to_node_sender, mut peer_to_node_receiver) = tokio::sync::mpsc::unbounded_channel();
    let node_to_peer_sender = Arc::new(node_to_peer_sender);
    let interrupt = Arc::new(AtomicI8::new(miner::worker::START));

    // TODO; Refactor [Directory]

    let node_id = NodeIdentity::from_config(
        NodeIdentityConfig::open(
            env.identity_file
                .clone()
                .unwrap_or_else(|| env.datadir.join("identity.json")),
        )
        .expect("identity file not found"),
    )
    .expect("failed to read identity file");

    let database = Arc::new(rocksdb::DB::open_cf_descriptors(
        &default_db_opts(),
        env.datadir.join("main"),
        column_families(),
    )?);
    let storage = Arc::new(PersistentStorage::new(PersistentStorageBackend::RocksDB(
        database,
    )));
    let consensus = Arc::new(BarossaProtocol::new(env.network));
    let blockchain = Arc::new(
        Chain::initialize(
            env.datadir.clone(),
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
        SyncService::new(
            blockchain.chain_state(),
            blockchain.txpool(),
            node_to_peer_sender.clone(),
            consensus,
            blockchain.chain_state().block_storage(),
            Arc::new(SyncMode::Normal),
        )
    };

    let identity_expected_pow = crypto::make_target(env.expected_pow);

    start_p2p_server(
        env.clone(),
        node_id,
        node_to_peer_receiver,
        peer_to_node_sender,
        env.peers.clone(),
        identity_expected_pow,
        network_state.clone(),
        blockchain.chain_state(),
        handler,
    )
    .await?;

    {
        let blockchain = blockchain.clone();
        let env = env.clone();
        tokio::spawn(start_rpc_server(
            local_mpsc_sender.clone(),
            blockchain.chain_state(),
            blockchain.chain_state().state(),
            blockchain.txpool(),
            env,
        ));
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
                blockchain.chain_state().vm(),
                blockchain.txpool(),
                blockchain.chain_state(),
                network_state,
                blockchain.chain_state().block_storage(),
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
                        Msg::BroadcastTransaction(msg) => {
                            let txpool = blockchain.txpool();
                            let mut txpool = txpool.write().unwrap();
                            txpool.add_remotes(msg.tx).unwrap()
                        }
                        Msg::BroadcastBlock(msg) => {
                            if let Some(block) = msg.block {
                                // TODO: validate block
                                // TODO: Check if future block is not further than 3 days
                                blockchain.chain_state().block_storage().put(block)?;
                            }
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
                            Msg::BroadcastBlock(BroadcastBlockMessage::new(block.clone())),
                        )
                        .unwrap();
                    }
                    LocalEventMessage::BroadcastTx(tx) => {
                        broadcast_message(
                            &node_to_peer_sender,
                            Msg::BroadcastTransaction(BroadcastTransactionMessage::new(tx)),
                        )
                        .unwrap();
                    }
                    LocalEventMessage::StateChanged { current_head } => {
                        broadcast_message(
                            &node_to_peer_sender,
                            Msg::CurrentHead(CurrentHeadMessage::new(current_head)),
                        )
                        .unwrap();
                    }
                    LocalEventMessage::NetworkNewPeerConnection { stats, .. } => {
                        info!(pending = ?stats.0, connected = ?stats.1, "Peers");
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

pub(crate) fn setup_environment(args: &RunArgs) -> Result<Arc<EnvironmentConfig>> {
    let mut config = EnvironmentConfig::default();

    if let Some(datadir) = &args.datadir {
        config.datadir = datadir.clone();
    }

    if let Some(config_file_path) = &args.config_file {
        let config_file = OpenOptions::new()
            .read(true)
            .open(config_file_path.as_path())?;
        config = serde_json::from_reader(config_file)?;
    } else {
        let res: Result<EnvironmentConfig, _> = OpenOptions::new()
            .read(true)
            .open(config.datadir.join("config.json"))
            .map_err(|e| anyhow::anyhow!("{}", e))
            .and_then(|config_file| {
                serde_json::from_reader(config_file).map_err(|e| anyhow::anyhow!("{}", e))
            });

        match res {
            Ok(c) => {
                config = c;
            }
            Err(error) => {
                warn!(error = ?error, "failed to read config file, reverting to application default");
            }
        }
    }

    if let Some(network) = args.network {
        config.network = network;
    }

    if !args.peer.is_empty() {
        config.peers = args.peer.clone();
    }

    if let Some(network) = args.network {
        config.network = network;
    }

    if let Some(coinbase) = args.miner {
        config.miner = Some(coinbase)
    }

    if let Some(expected_pow) = args.expected_pow {
        config.expected_pow = expected_pow
    }

    if let Some(p2p_host) = &args.p2p_host {
        config.p2p_host = p2p_host.clone()
    }

    if let Some(rpc_host) = &args.rpc_host {
        config.rpc_host = rpc_host.clone()
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

    config.sanitize();

    Ok(Arc::new(config))
}
