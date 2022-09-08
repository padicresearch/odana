use std::sync::Arc;

use tonic::{Code, Request, Response, Status};

use primitive_types::H256;
use proto::rpc::chain_service_server::ChainService;
use proto::rpc::{
    CurrentHeadResponse, GetBlockByHashRequest, GetBlockByLevelRequest, GetBlockNumberResponse,
    GetBlocksRequest, GetBlocksResponse,
};
use traits::Blockchain;
use types::block::Block;
use types::prelude::Empty;

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
    ) -> Result<Response<CurrentHeadResponse>, Status> {
        let indexed_blockheader = self
            .blockchain
            .current_header()
            .map_err(|_e| Status::new(Code::NotFound, "head not available"))?;

        let blockheader =
            indexed_blockheader.ok_or_else(|| Status::new(Code::NotFound, "head not available"))?;

        let hash = blockheader.hash.as_bytes().to_vec();
        Ok(Response::new(CurrentHeadResponse {
            hash,
            header: Some(blockheader.raw),
        }))
    }

    async fn block_level(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<GetBlockNumberResponse>, Status> {
        let ch = self.current_head(request).await?;
        let inner = ch.into_inner();
        Ok(Response::new(GetBlockNumberResponse {
            level: inner.header.map(|head| head.level()).unwrap_or_default(),
        }))
    }

    async fn get_block_by_hash(
        &self,
        request: Request<GetBlockByHashRequest>,
    ) -> Result<Response<Block>, Status> {
        let raw_hash = request.into_inner().hash;
        let block_hash = H256::from_slice(&raw_hash);
        let block = self
            .blockchain
            .get_block_by_hash(&block_hash)
            .map_err(|_| Status::internal(""))?;
        let block = block.ok_or_else(|| Status::not_found(format!("Block hash {}", block_hash)))?;
        Ok(Response::new(block))
    }

    async fn get_block_by_level(
        &self,
        request: Request<GetBlockByLevelRequest>,
    ) -> Result<Response<Block>, Status> {
        let level = request.into_inner().level;
        let block = self
            .blockchain
            .get_block_by_level(level)
            .map_err(|_| Status::internal(""))?;
        let block = block.ok_or_else(|| Status::not_found(format!("Block level {}", level)))?;
        Ok(Response::new(block))
    }

    async fn get_blocks(
        &self,
        _: Request<GetBlocksRequest>,
    ) -> Result<Response<GetBlocksResponse>, Status> {
        todo!()
    }
}
