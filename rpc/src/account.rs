use std::str::FromStr;
use std::sync::Arc;
use tonic::{Code, Request, Response, Status};
use primitive_types::H160;
use proto::rpc::account_service_server::AccountService;
use proto::rpc::{GetAccountBalanceRequest, GetAccountBalanceResponse};
use traits::StateDB;

pub(crate) struct AccountServiceImpl {
    state: Arc<dyn StateDB>,
}

impl AccountServiceImpl {
    pub(crate) fn new(state: Arc<dyn StateDB>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AccountService for AccountServiceImpl {
    async fn get_account_balance(&self, request: Request<GetAccountBalanceRequest>) -> Result<Response<GetAccountBalanceResponse>, Status> {
        let req = request.into_inner();
        let address = H160::from_str(&req.address).map_err(|e| Status::new(Code::InvalidArgument, "Invalid Request"))?;
        let balance = self.state.balance(&address);
        Ok(Response::new(GetAccountBalanceResponse {
            balance: balance.to_string(),
        }))
    }
}