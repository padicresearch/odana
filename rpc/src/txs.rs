use std::sync::{Arc, RwLock};

use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

use primitive_types::H256;
use proto::rpc::transactions_service_server::TransactionsService;
use proto::rpc::{
    AddressTransactionList, GetTransactionStatusResponse, PendingTransactionsResponse,
    SignedTransactionResponse, TransactionHash, TransactionHashes, TxpoolContentResponse,
    UnsignedTransactionRequest,
};
use tracing::warn;
use txpool::TxPool;
use types::account::get_address_from_secret_key;
use types::events::LocalEventMessage;
use types::network::Network;
use types::prelude::Empty;
use types::tx::SignedTransaction;

pub(crate) struct TransactionsServiceImpl {
    txpool: Arc<RwLock<TxPool>>,
    sender: UnboundedSender<LocalEventMessage>,
}

impl TransactionsServiceImpl {
    pub(crate) fn new(
        txpool: Arc<RwLock<TxPool>>,
        sender: UnboundedSender<LocalEventMessage>,
    ) -> Self {
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
        let request = request.into_inner();
        let secret_key = request.secret_key;
        let mut tx = request
            .tx
            .ok_or_else(|| Status::invalid_argument("tx arg not found or failed to decode"))?;
        let address = get_address_from_secret_key(
            H256::from_slice(&secret_key),
            Network::from_chain_id(tx.chain_id),
        )
        .map_err(|e| Status::internal(e.to_string()))?;
        if tx.nonce == 0 {
            tx.nonce = txpool.nonce(&address);
        }

        let signed_tx = transaction::sign_tx(H256::from_slice(&secret_key), tx)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SignedTransactionResponse {
            hash: signed_tx.hash().into(),
            tx: Some(signed_tx),
        }))
    }

    async fn sign_send_transaction(
        &self,
        request: Request<UnsignedTransactionRequest>,
    ) -> Result<Response<SignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let tx = req
            .tx
            .ok_or_else(|| Status::invalid_argument("tx arg not found or failed to decode"))?;
        let signed_tx = transaction::sign_tx(H256::from_slice(&req.secret_key), tx)
            .map_err(|e| Status::internal(e.to_string()))?;
        let tx_hash = signed_tx.hash_256();
        let mut txpool = self.txpool.write().map_err(|_| Status::internal(""))?;
        txpool
            .add_local(signed_tx.clone())
            .map_err(|e| Status::unknown(e.to_string()))?;

        self.sender
            .send(LocalEventMessage::BroadcastTx(vec![signed_tx]))
            .map_err(|_| {
                warn!(tx_hash = ?tx_hash, "failed to send tx to peers");
                Status::internal("")
            })?;

        Ok(Response::new(SignedTransactionResponse {
            hash: tx_hash.as_bytes().to_vec(),
            tx: None,
        }))
    }

    async fn send_transaction(
        &self,
        request: Request<SignedTransaction>,
    ) -> Result<Response<TransactionHash>, Status> {
        let signed_tx = request.into_inner();
        let tx_hash = signed_tx.hash().to_vec();
        let mut txpool = self.txpool.write().map_err(|_| Status::internal(""))?;
        txpool
            .add_local(signed_tx.clone())
            .map_err(|e| Status::aborted(e.to_string()))?;

        self.sender
            .send(LocalEventMessage::BroadcastTx(vec![signed_tx]))
            .map_err(|_| {
                warn!(tx_hash = ?tx_hash, "failed to send tx to peers");
                Status::internal("")
            })?;

        Ok(Response::new(TransactionHash { hash: tx_hash }))
    }

    async fn get_transaction_status(
        &self,
        request: Request<TransactionHashes>,
    ) -> Result<Response<GetTransactionStatusResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let txs: Vec<_> = request
            .get_ref()
            .txs
            .iter()
            .map(|tx_hash| H256::from_slice(tx_hash))
            .collect();
        let status: Vec<_> = txpool
            .status(txs)
            .iter()
            .copied()
            .map(|tx_status| tx_status as i32)
            .collect();
        Ok(Response::new(GetTransactionStatusResponse { status }))
    }

    async fn get_pending_transactions(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<PendingTransactionsResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let pending: Vec<_> = txpool
            .pending()
            .iter()
            .map(|(address, txs)| AddressTransactionList {
                address: address.to_vec(),
                txs: Some(txs.clone()),
            })
            .collect();
        Ok(Response::new(PendingTransactionsResponse { pending }))
    }

    async fn get_txpool_content(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<TxpoolContentResponse>, Status> {
        let txpool = self.txpool.read().map_err(|_| Status::internal(""))?;
        let content = txpool.content();
        let pending: Vec<_> = content
            .0
            .iter()
            .map(|(address, txs)| AddressTransactionList {
                address: address.to_vec(),
                txs: Some(txs.clone()),
            })
            .collect();
        let queued: Vec<_> = content
            .1
            .iter()
            .map(|(address, txs)| AddressTransactionList {
                address: address.to_vec(),
                txs: Some(txs.clone()),
            })
            .collect();
        Ok(Response::new(TxpoolContentResponse { pending, queued }))
    }
}
