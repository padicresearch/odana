extern crate core;

use crate::rpc::account_service_client::AccountServiceClient;
use crate::rpc::chain_service_client::ChainServiceClient;
use crate::rpc::runtime_api_service_client::RuntimeApiServiceClient;
use crate::rpc::transactions_service_client::TransactionsServiceClient;
use tonic::codegen::StdError;

mod cmd;
pub mod commands;
#[allow(clippy::all)]
mod rpc;
mod util;
mod value;

pub struct Client {
    conn: tonic::transport::Channel,
}

impl Client {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
    {
        let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
        Ok(Self { conn })
    }

    pub fn transaction_service(&self) -> TransactionsServiceClient<tonic::transport::Channel> {
        TransactionsServiceClient::new(self.conn.clone())
    }

    pub fn account_service(&self) -> AccountServiceClient<tonic::transport::Channel> {
        AccountServiceClient::new(self.conn.clone())
    }

    pub fn blockchain_service(&self) -> ChainServiceClient<tonic::transport::Channel> {
        ChainServiceClient::new(self.conn.clone())
    }

    pub fn runtime_api_service(&self) -> RuntimeApiServiceClient<tonic::transport::Channel> {
        RuntimeApiServiceClient::new(self.conn.clone())
    }
}
