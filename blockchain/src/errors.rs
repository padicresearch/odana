use anyhow::Error;
use thiserror::Error;
use types::block::BlockHeader;

#[derive(Error, Debug)]
pub enum BlockChainError {
    #[error("VerifyError")]
    VerifyError,
    #[error("TxInputOrOutputLessThanEqualZero")]
    TxInputOrOutputEmpty,
    #[error("MerkleError")]
    MerkleError,
    #[error("RWPoison")]
    RWPoison,
    #[error("`{0}`")]
    HexError(#[from] hex::FromHexError),
    #[error("`{0}`")]
    GenericError(#[from] anyhow::Error),
    #[error("UTXOError `{0}`")]
    UTXOError(&'static str),
    #[error("InvalidTransactionFees")]
    InvalidTransactionFees,
    #[error("MemPoolTransactionNotFound")]
    MemPoolTransactionNotFound,
    #[error("TransactionFeeTooLow")]
    TransactionFeeTooLow,
    #[error("InvalidTransaction")]
    InvalidTransaction,
    #[error("InvalidTransactionCoinSpent")]
    InvalidTransactionCoinSpent,
    #[error("TransactionNotFound")]
    TransactionNotFound,
    #[error("InvalidBlock")]
    InvalidBlock,
    #[error("UnknownError")]
    UnknownError,
    #[error("Failed to verify header expected {0:#?} {1:#?} detail error {3}")]
    FailedToVerifyHeader(BlockHeader, BlockHeader, Error),
}
