use storage::{PersistentStorage, KVEntry};
use storage::sleddb::SledDB;
use std::sync::Arc;
use blockchain::blockchain::{BlockChain, start_mining, StateAction, BlockChainState, LocalMessage};
use blockchain::p2p::{start_p2p_server, NodeIdentity, PeerMessage, BroadcastBlockMessage, BroadcastTransactionMessage, CurrentHeadMessage};
use blockchain::block::Block;
use std::env;
use storage::memstore::MemStore;
use blockchain::block_storage::BlockStorage;
use blockchain::utxo::UTXO;
use blockchain::mempool::MemPool;
use anyhow::Error;
enum EventStream {
    LocalMessage(LocalMessage),
    PeerMessage(PeerMessage),
    Unhandled,
}
///tmp/tuchain
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //logging
    common::tracing_subscriber::fmt::init();

    // Communications
    let (local_mpsc_sender, mut local_mpsc_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (node_2_peer_sender, mut node_2_peer_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (peer_2_node_sender, mut peer_2_node_receiver) = tokio::sync::mpsc::unbounded_channel();


    let kv = Arc::new(MemStore::new(vec![BlockStorage::column(), UTXO::column(), MemPool::column(), BlockChainState::column()]));
    let storage = Arc::new(PersistentStorage::InMemory(kv));
    let blockchain = BlockChain::new(storage, local_mpsc_sender.clone())?;



    start_mining(blockchain.miner(), blockchain.state(), local_mpsc_sender);
    start_p2p_server(NodeIdentity::generate(), node_2_peer_receiver, peer_2_node_sender).await?;

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
                            if let (Ok(Some(current_head)),Ok(mempool))  = (blockchain.state().get_current_head(),blockchain.state().get_mempool()) {
                                node_2_peer_sender.send(PeerMessage::CurrentHead(CurrentHeadMessage::new(current_head, mempool, Some(req.sender))));
                            }
                        }
                        PeerMessage::CurrentHead(msg) => {
                            println!("Received CurrentHead {:?}", msg);
                        }
                        PeerMessage::GetBlockHeader(_) => {}
                        PeerMessage::BlockHeader(_) => {}
                        PeerMessage::GetBlockTransactions(_) => {}
                        PeerMessage::BlockTransactions(_) => {}
                        PeerMessage::BroadcastTransaction(tx_msg) => {
                            match blockchain.dispatch(StateAction::AddNewTransaction(tx_msg.tx())) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("State Dispatch Error {}", e)
                                }
                            };
                        }
                        PeerMessage::BroadcastBlock(block_msg) => {
                            match blockchain.dispatch(StateAction::AddNewBlock(block_msg.block())) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("State Dispatch Error {}", e)
                                }
                            };

                        }
                    };

                }
                EventStream::LocalMessage(local_msg) => {
                    match local_msg {
                        LocalMessage::MindedBlock(block) => {
                           println!("Minded new Block : {}", block);
                            node_2_peer_sender.send(PeerMessage::BroadcastBlock(BroadcastBlockMessage::new(block.clone())));
                            match blockchain.dispatch(StateAction::AddNewBlock(block)) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("State Dispatch Error {}", e)
                                }
                            };
                        }
                        LocalMessage::BroadcastTx(tx) => {


                            match blockchain.dispatch(StateAction::AddNewTransaction(tx.clone())) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("State Dispatch Error {}", e)
                                }
                            };
                            //println!("Sending Transaction to Chain : {:?}", tx);
                            node_2_peer_sender.send(PeerMessage::BroadcastTransaction(BroadcastTransactionMessage::new(tx)));
                        }
                        LocalMessage::StateChanged { current_head, mempool } => {
                            node_2_peer_sender.send(PeerMessage::CurrentHead(CurrentHeadMessage::new(current_head, mempool, None)));
                        }
                    }
                }
                EventStream::Unhandled => {}
            }

        }


    }
}
