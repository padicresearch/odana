use std::sync::Arc;
use tonic::{Response, Status, Request, Code};
use proto::blockchain_rpc_service_server::{BlockchainRpcService};
use proto::{Empty, GetBlockNumberResponse, GetBlockRequest, GetBlockResponse, GetBlocksRequest, GetBlocksResponse, GetHeadResponse};
use traits::Blockchain;

struct BlockChainRPCServiceImpl {
    blockchain: Arc<dyn Blockchain>,
}

#[tonic::async_trait]
impl BlockchainRpcService for BlockChainRPCServiceImpl {
    async fn get_head(&self, request: Request<Empty>) -> Result<Response<GetHeadResponse>, Status> {
        let ch = self.blockchain.current_header().map_err(|e| Status::new(Code::NotFound, "head not available"))?;
        let ch = ch.ok_or(Status::new(Code::NotFound, "head not available"))?;
        todo!()
    }

    async fn get_block_number(&self, request: Request<Empty>) -> Result<Response<GetBlockNumberResponse>, Status> {
        todo!()
    }

    async fn get_block(&self, request: Request<GetBlockRequest>) -> Result<Response<GetBlockResponse>, Status> {
        todo!()
    }

    async fn get_blocks(&self, request: Request<GetBlocksRequest>) -> Result<Response<GetBlocksResponse>, Status> {
        todo!()
    }
}