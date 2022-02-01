use thiserror::Error;

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
}
