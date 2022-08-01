mod blockchain;

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info};
use anyhow::Result;
use tonic::transport::Server;
use proto::blockchain_rpc_service_server::BlockchainRpcServiceServer;
use traits::Blockchain;
use types::config::EnvironmentConfig;
use crate::blockchain::BlockChainRPCServiceImpl;

pub struct RPC;

pub async fn start_rpc_server(blockchain: Arc<dyn Blockchain>, env: Arc<EnvironmentConfig>) -> Result<()> {
    let host = env.host();
    let port = env.rpc_port();
    let addr = SocketAddr::new(host.parse()?, port);
    let blockchain_rpc = BlockChainRPCServiceImpl::new(blockchain);

    info!(addr = ?addr, "RPC server running at");
    Server::builder()
        .add_service(BlockchainRpcServiceServer::new(blockchain_rpc))
        .serve(addr)
        .await?;
    Ok(())
}