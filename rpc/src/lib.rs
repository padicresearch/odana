mod account;
mod blockchain;
mod txs;

use crate::account::AccountServiceImpl;
use crate::blockchain::ChainServiceImpl;
use crate::txs::TransactionsServiceImpl;
use anyhow::Result;
use proto::rpc::account_service_server::AccountServiceServer;
use proto::rpc::chain_service_server::ChainServiceServer;
use proto::rpc::transactions_service_server::TransactionsServiceServer;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tonic::transport::Server;
use tracing::info;
use traits::{Blockchain, StateDB};
use txpool::TxPool;
use types::config::EnvironmentConfig;
use types::events::LocalEventMessage;

pub struct RPC;

pub async fn start_rpc_server(
    n2p_sender: UnboundedSender<LocalEventMessage>,
    blockchain: Arc<dyn Blockchain>,
    state: Arc<dyn StateDB>,
    txpool: Arc<RwLock<TxPool>>,
    env: Arc<EnvironmentConfig>,
) -> Result<()> {
    let host = env.host();
    let port = env.rpc_port();
    let addr = SocketAddr::new(host.parse()?, port);
    let chain_service = ChainServiceImpl::new(blockchain);
    let account_service = AccountServiceImpl::new(state, txpool.clone());
    let transaction_service = TransactionsServiceImpl::new(txpool,n2p_sender);
    info!(addr = ?addr, "RPC server running at");
    Server::builder()
        .add_service(ChainServiceServer::new(chain_service))
        .add_service(AccountServiceServer::new(account_service))
        .add_service(TransactionsServiceServer::new(transaction_service))
        .serve(addr)
        .await?;
    Ok(())
}
