use anyhow::Result;
use proto::blockchain_rpc_service_server::{BlockchainRpcService, BlockchainRpcServiceServer};
use proto::{
    BlockHeader, Empty, GetBlockByHashRequest, GetBlockByLevelRequest, GetBlockNumberResponse,
    GetBlockRequest, GetBlockResponse, GetBlocksRequest, GetBlocksResponse, GetHeadResponse,
};
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status};
use traits::Blockchain;

pub(crate) struct BlockChainRPCServiceImpl {
    blockchain: Arc<dyn Blockchain>,
}

impl BlockChainRPCServiceImpl {
    pub(crate) fn new(blockchain: Arc<dyn Blockchain>) -> Self {
        Self { blockchain }
    }
}

#[tonic::async_trait]
impl BlockchainRpcService for BlockChainRPCServiceImpl {
    async fn current_head(
        &self,
        request: Request<Empty>,
    ) -> std::result::Result<Response<BlockHeader>, Status> {
        let ch = self
            .blockchain
            .current_header()
            .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        let head: BlockHeader = ch
            .ok_or(Status::new(Code::NotFound, "head not available"))?
            .raw
            .into()
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
    // async fn get_head(&self, _: Request<Empty>) -> Result<Response<GetHeadResponse>, Status> {
    //     let ch = self
    //         .blockchain
    //         .current_header()
    //         .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
    //     let head: BlockHeader = ch
    //         .ok_or(Status::new(Code::NotFound, "head not available"))?
    //         .raw
    //         .into();
    //     Ok(Response::new(GetHeadResponse { block: Some(head) }))
    // }
    //
    // async fn get_block_number(
    //     &self,
    //     _: Request<Empty>,
    // ) -> Result<Response<GetBlockNumberResponse>, Status> {
    //     let ch = self
    //         .blockchain
    //         .current_header()
    //         .map_err(|e| Status::new(Code::NotFound, "head not available"))?;
    //     let head = ch
    //         .ok_or(Status::new(Code::NotFound, "head not available"))?
    //         .raw;
    //     Ok(Response::new(GetBlockNumberResponse { level:  head.level}))
    // }
    //
    // async fn get_block(
    //     &self,
    //     request: Request<GetBlockRequest>,
    // ) -> Result<Response<GetBlockResponse>, Status> {
    //     todo!()
    // }
    //
    // async fn get_blocks(
    //     &self,
    //     request: Request<GetBlocksRequest>,
    // ) -> Result<Response<GetBlocksResponse>, Status> {
    //     todo!()
    // }
}
