use std::str::FromStr;
use std::sync::{Arc, RwLock};

use tonic::{Code, Request, Response, Status};

use primitive_types::H160;
use proto::rpc::account_service_server::AccountService;
use proto::rpc::{GetAccountBalanceResponse, GetAccountNonceResponse, GetAccountRequest};
use proto::AccountState;
use traits::StateDB;
use txpool::TxPool;

pub(crate) struct AccountServiceImpl {
    state: Arc<dyn StateDB>,
    txpool: Arc<RwLock<TxPool>>,
}

impl AccountServiceImpl {
    pub(crate) fn new(state: Arc<dyn StateDB>, txpool: Arc<RwLock<TxPool>>) -> Self {
        Self { state, txpool }
    }
}

#[tonic::async_trait]
impl AccountService for AccountServiceImpl {
    async fn get_balance(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountBalanceResponse>, Status> {
        let req = request.into_inner();
        let address = H160::from_str(&req.address)
            .map_err(|_| Status::new(Code::InvalidArgument, "Invalid Request"))?;
        let balance = self.state.balance(&address);
        Ok(Response::new(GetAccountBalanceResponse {
            balance: hex::encode(balance, true),
        }))
    }

    async fn get_nonce(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountNonceResponse>, Status> {
        let req = request.into_inner();
        let address = H160::from_str(&req.address)
            .map_err(|_| Status::new(Code::InvalidArgument, "Invalid Request"))?;
        let txpool = self.txpool.read().unwrap();
        let nonce = txpool.nonce(&address);
        Ok(Response::new(GetAccountNonceResponse {
            nonce: format!("{:#02x}", nonce),
        }))
    }

    async fn get_account_state(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<AccountState>, Status> {
        let req = request.into_inner();
        let address = H160::from_str(&req.address)
            .map_err(|_| Status::new(Code::InvalidArgument, "Invalid Request"))?;
        let account_state = self.state.account_state(&address);
        Ok(Response::new(account_state.into_proto().unwrap()))
    }
}
