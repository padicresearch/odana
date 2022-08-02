mod blockchain;
mod account;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info};
use anyhow::Result;
use tonic::transport::Server;
use proto::rpc::account_service_server::AccountServiceServer;
use proto::rpc::chain_service_server::ChainServiceServer;
use traits::{Blockchain, StateDB};
use types::config::EnvironmentConfig;
use crate::account::AccountServiceImpl;
use crate::blockchain::ChainServiceImpl;

pub struct RPC;

pub async fn start_rpc_server(blockchain: Arc<dyn Blockchain>, state: Arc<dyn StateDB>, env: Arc<EnvironmentConfig>) -> Result<()> {
    let host = env.host();
    let port = env.rpc_port();
    let addr = SocketAddr::new(host.parse()?, port);
    let chain_service = ChainServiceImpl::new(blockchain);
    let account_service = AccountServiceImpl::new(state);

    info!(addr = ?addr, "RPC server running at");
    Server::builder()
        .add_service(ChainServiceServer::new(chain_service))
        .add_service(AccountServiceServer::new(account_service))
        .serve(addr)
        .await?;
    Ok(())
}