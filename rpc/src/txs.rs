use anyhow::Error;
use primitive_types::{H160, H256, U128};
use proto::rpc::account_service_server::AccountService;
use proto::rpc::transactions_service_server::TransactionsService;
use proto::rpc::{
    GetAccountBalanceResponse, GetTransactionStatusResponse, PendingTransactionsResponse,
    SignedTransactionResponse, TransactionHash, TransactionHashes, TxpoolContentResponse,
    UnsignedTransactionRequest,
};
use proto::{Empty, Transaction, UnsignedTransaction};
use std::collections::HashMap;
use std::fmt::format;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Code, Request, Response, Status};
use tracing::warn;
use traits::StateDB;
use txpool::TxPool;
use types::account::get_address_from_secret_key;
use types::events::LocalEventMessage;
use types::tx::SignedTransaction;

pub(crate) struct TransactionsServiceImpl {
    txpool: Arc<RwLock<TxPool>>,
    sender: UnboundedSender<LocalEventMessage>,
}

impl TransactionsServiceImpl {
    pub(crate) fn new(txpool: Arc<RwLock<TxPool>>, sender: UnboundedSender<LocalEventMessage>) -> Self {
        Self { txpool, sender }
    }
}

#[tonic::async_trait]
impl TransactionsService for TransactionsServiceImpl {
    async fn sign_transaction(
        &self,
        request: Request<UnsignedTransactionRequest>,
    ) -> Result<Response<SignedTransactionResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let req = request.into_inner();
        let mut tx = req.tx.ok_or(Status::invalid_argument(
            "tx arg not found or failed to decode",
        ))?;
        let address = get_address_from_secret_key(
            H256::from_str(&req.key).map_err(|e| Status::internal(e.to_string()))?,
        ).map_err(|e| Status::internal(e.to_string()))?;
        let nonce = U128::from_str(&tx.nonce).unwrap_or_default();
        if nonce.as_u128() == 0 {
            tx.nonce = prefix_hex::encode(U128::from(txpool.nonce(&address)));
        }
        let signed_tx =
            transaction::sign_tx(H256::from_str(&req.key).unwrap_or_default(), tx.clone())
                .map_err(|e| Status::internal(format!("{}", e)))?;
        Ok(Response::new(SignedTransactionResponse {
            hash: format!("{:?}", signed_tx.hash_256()),
            tx: signed_tx
                .into_proto()
                .map(|tx| Some(tx))
                .map_err(|e| Status::internal(format!("{}", e)))?,
        }))
    }

    async fn sign_send_transaction(
        &self,
        request: Request<UnsignedTransactionRequest>,
    ) -> Result<Response<SignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let mut tx = req.tx.ok_or(Status::invalid_argument(
            "tx arg not found or failed to decode",
        ))?;
        let signed_tx =
            transaction::sign_tx(H256::from_str(&req.key).unwrap_or_default(), tx.clone())
                .map_err(|e| Status::internal(format!("{}", e)))?;
        let tx_hash = signed_tx.hash_256();
        let mut txpool = self.txpool.write().map_err(|_| Status::internal(""))?;
        txpool
            .add_local(signed_tx.clone())
            .map_err(|e| Status::aborted(format!("{}", e)))?;

        self.sender.send(LocalEventMessage::BroadcastTx(vec![signed_tx])).map_err(|_| {
            warn!(tx_hash = ?tx_hash, "failed to send tx to peers");
            Status::internal("")
        })?;

        Ok(Response::new(SignedTransactionResponse {
            hash: format!("{:?}", tx_hash),
            tx: None,
        }))
    }

    async fn send_transaction(
        &self,
        request: Request<Transaction>,
    ) -> Result<Response<TransactionHash>, Status> {
        let signed_tx = SignedTransaction::from_proto(request.into_inner())
            .map_err(|_| Status::internal(""))?;
        let tx_hash = signed_tx.hash_256();
        let mut txpool = self.txpool.write().map_err(|_| Status::internal(""))?;
        txpool
            .add_local(signed_tx.clone())
            .map_err(|e| Status::aborted(format!("{}", e)))?;

        self.sender.send(LocalEventMessage::BroadcastTx(vec![signed_tx])).map_err(|_| {
            warn!(tx_hash = ?tx_hash, "failed to send tx to peers");
            Status::internal("")
        })?;

        Ok(Response::new(TransactionHash {
            hash: format!("{:?}", tx_hash),
        }))
    }

    async fn get_transaction_status(
        &self,
        request: Request<TransactionHashes>,
    ) -> Result<Response<GetTransactionStatusResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let json_rep = serde_json::to_vec(&request.into_inner().tx_hashes)
            .map_err(|_| Status::internal(""))?;
        let txs: Vec<H256> = serde_json::from_slice(&json_rep).unwrap_or_default();
        let status = txpool.status(txs);
        let json_rep = serde_json::to_vec(&status).map_err(|_| Status::internal(""))?;
        let tx_status = serde_json::from_slice(&json_rep).map_err(|_| Status::internal(""))?;
        Ok(Response::new(GetTransactionStatusResponse {
            status: tx_status,
        }))
    }

    async fn get_pending_transactions(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<PendingTransactionsResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let pending = txpool.pending();
        let json_rep = serde_json::to_vec(&pending).unwrap();
        let pending: HashMap<String, proto::TransactionList> =
            serde_json::from_slice(&json_rep).unwrap_or_default();
        Ok(Response::new(PendingTransactionsResponse { pending }))
    }

    async fn get_txpool_content(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<TxpoolContentResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let content = txpool.content();
        let json_rep = serde_json::to_vec(&content.0)
            .map_err(|_| Status::internal("Error parsing message"))?;
        let pending: HashMap<String, proto::TransactionList> =
            serde_json::from_slice(&json_rep).unwrap_or_default();
        let json_rep = serde_json::to_vec(&content.1)
            .map_err(|_| Status::internal("Error parsing message"))?;
        let queued: HashMap<String, proto::TransactionList> =
            serde_json::from_slice(&json_rep).unwrap_or_default();
        Ok(Response::new(TxpoolContentResponse { pending, queued }))
    }
}
