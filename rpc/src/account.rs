use std::sync::{Arc, RwLock};

use tonic::{Request, Response, Status};

use crate::rpc::account_service_server::AccountService;
use crate::rpc::{GetAccountBalanceResponse, GetAccountNonceResponse, GetAccountRequest};
use traits::StateDB;
use txpool::TxPool;
use types::account::AccountState;

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
        let req = request.get_ref();
        let address = req
            .address
            .ok_or_else(|| Status::unknown("failed to parse address"))?;
        let balance = self.state.balance(&address);
        Ok(Response::new(GetAccountBalanceResponse { balance }))
    }

    async fn get_nonce(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountNonceResponse>, Status> {
        let req = request.into_inner();
        let address = req
            .address
            .ok_or_else(|| Status::unknown("failed to parse address"))?;
        let txpool = self.txpool.read().unwrap();
        let nonce = txpool.nonce(&address);
        Ok(Response::new(GetAccountNonceResponse { nonce }))
    }

    async fn get_account_state(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<AccountState>, Status> {
        let req = request.into_inner();
        let address = req
            .address
            .ok_or_else(|| Status::unknown("failed to parse address"))?;
        let account_state = self.state.account_state(&address);
        Ok(Response::new(account_state))
    }
}
