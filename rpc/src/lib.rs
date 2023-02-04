use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tonic::transport::Server;

use crate::rpc::account_service_server::AccountServiceServer;
use crate::rpc::chain_service_server::ChainServiceServer;
use crate::rpc::transactions_service_server::TransactionsServiceServer;
use tracing::info;
use traits::{Blockchain, StateDB, WasmVMInstance};
use txpool::TxPool;
use types::config::EnvironmentConfig;
use types::events::LocalEventMessage;

use crate::account::AccountServiceImpl;
use crate::blockchain::ChainServiceImpl;
use crate::rpc::runtime_api_service_server::RuntimeApiServiceServer;
use crate::runtime::RuntimeApiServiceImpl;
use crate::txs::TransactionsServiceImpl;

mod account;
mod blockchain;
mod rpc;
mod txs;
mod runtime;

pub struct RPC;

pub async fn start_rpc_server(
    n2p_sender: UnboundedSender<LocalEventMessage>,
    blockchain: Arc<dyn Blockchain>,
    vm: Arc<dyn WasmVMInstance>,
    state: Arc<dyn StateDB>,
    txpool: Arc<RwLock<TxPool>>,
    env: Arc<EnvironmentConfig>,
) -> Result<()> {
    let host = env.rpc_host();
    let port = env.rpc_port();
    let addr = SocketAddr::new(host.parse()?, port);
    let chain_service = ChainServiceImpl::new(blockchain);
    let account_service = AccountServiceImpl::new(state.clone(), txpool.clone());
    let transaction_service = TransactionsServiceImpl::new(txpool, n2p_sender);
    let rt_api_service = RuntimeApiServiceImpl::new(state, vm);
    info!(addr = ?addr, "RPC server running at");
    Server::builder()
        .add_service(ChainServiceServer::new(chain_service))
        .add_service(AccountServiceServer::new(account_service))
        .add_service(TransactionsServiceServer::new(transaction_service))
        .add_service(RuntimeApiServiceServer::new(rt_api_service))
        .serve(addr)
        .await?;
    Ok(())
}
