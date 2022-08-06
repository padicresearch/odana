use std::fmt::format;
use anyhow::Result;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status};
use primitive_types::{H160, H256};
use proto::{Block, BlockHeader, Empty};
use proto::rpc::chain_service_server::ChainService;
use proto::rpc::{CurrentHeadResponse, GetBlockByHashRequest, GetBlockByLevelRequest, GetBlockNumberResponse, GetBlocksRequest, GetBlocksResponse};
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
    ) -> std::result::Result<Response<CurrentHeadResponse>, Status> {
        let indexed_blockheader = self
            .blockchain
            .current_header()
            .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        let hash = indexed_blockheader.as_ref().map(|head| format!("{:?}", head.hash)).unwrap_or_default();
        let header: BlockHeader = indexed_blockheader
            .ok_or(Status::new(Code::NotFound, "head not available"))?
            .raw
            .into_proto()
            .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        Ok(Response::new(CurrentHeadResponse {
            hash,
            header: Some(header),
        }))
    }

    async fn block_level(
        &self,
        request: Request<Empty>,
    ) -> std::result::Result<Response<GetBlockNumberResponse>, Status> {
        let ch = self.current_head(request).await?;
        let inner = ch.into_inner();
        Ok(Response::new(GetBlockNumberResponse { level: inner.header.map(|head| head.level).unwrap_or_default() }))
    }

    async fn get_block_by_hash(&self, request: Request<GetBlockByHashRequest>) -> std::result::Result<Response<Block>, Status> {
        let raw_hash = request.into_inner().hash;
        let block_hash = H256::from_str(&raw_hash).map_err(|err| Status::invalid_argument(format!("{}", err)))?;
        let block = self.blockchain.get_block_by_hash(&block_hash).map_err(|_| Status::internal(""))?;
        let block = block.ok_or(Status::not_found(format!("Bloc hash {}", block_hash)))?;
        Ok(Response::new(block.into_proto().map_err(|e| Status::internal(e.to_string()))?))
    }

    async fn get_block_by_level(&self, request: Request<GetBlockByLevelRequest>) -> std::result::Result<Response<Block>, Status> {
        let level = request.into_inner().level;
        let block = self.blockchain.get_block_by_level(level).map_err(|_| Status::internal(""))?;
        let block = block.ok_or(Status::not_found(format!("Block level {}", level)))?;
        Ok(Response::new(block.into_proto().map_err(|e| Status::internal(e.to_string()))?))
    }

    async fn get_blocks(&self, request: Request<GetBlocksRequest>) -> std::result::Result<Response<GetBlocksResponse>, Status> {
        todo!()
    }
}
