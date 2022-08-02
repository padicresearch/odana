use std::fmt::format;
use anyhow::Result;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status};
use primitive_types::H160;
use proto::{BlockHeader, Empty};
use proto::rpc::chain_service_server::ChainService;
use proto::rpc::{GetBlockByHashRequest, GetBlockByLevelRequest, GetBlockNumberResponse, GetBlockResponse, GetBlocksRequest, GetBlocksResponse};
use traits::{Blockchain, StateDB};

pub(crate) struct ChainServiceImpl {
    blockchain: Arc<dyn Blockchain>,
}

impl ChainServiceImpl {
    pub(crate) fn new(blockchain: Arc<dyn Blockchain>) -> Self {
        Self { blockchain }
    }
}

#[tonic::async_trait]
impl ChainService for ChainServiceImpl {
    async fn current_head(
        &self,
        _: Request<Empty>,
    ) -> std::result::Result<Response<BlockHeader>, Status> {
        let ch = self
            .blockchain
            .current_header()
            .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        let head: BlockHeader = ch
            .ok_or(Status::new(Code::NotFound, "head not available"))?
            .raw
            .into_proto()
            .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        Ok(Response::new(head))
    }

    async fn block_level(
        &self,
        request: Request<Empty>,
    ) -> std::result::Result<Response<GetBlockNumberResponse>, Status> {
        let ch = self.current_head(request).await?;
        let inner = ch.into_inner();
        Ok(Response::new(GetBlockNumberResponse { level: inner.level }))
    }

    async fn get_block_by_hash(
        &self,
        request: Request<GetBlockByHashRequest>,
    ) -> std::result::Result<Response<GetBlockResponse>, Status> {
        todo!()
    }

    async fn get_block_by_level(
        &self,
        request: Request<GetBlockByLevelRequest>,
    ) -> std::result::Result<Response<GetBlockResponse>, Status> {
        todo!()
    }

    async fn get_blocks(
        &self,
        request: Request<GetBlocksRequest>,
    ) -> std::result::Result<Response<GetBlocksResponse>, Status> {
        todo!()
    }
}
