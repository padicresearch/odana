use thiserror::Error;

#[derive(Error, Debug)]
pub enum TxPoolError {
    #[error("RWPoison")]
    RWPoison,
    #[error("MutexGuard error {0}")]
    MutexGuardError(String),
    #[error("`{0}`")]
    GenericError(#[from] anyhow::Error),
    #[error("`{0}`")]
    SqliteError(#[from] rusqlite::Error),
    #[error("`Transaction is already known`")]
    TransactionAlreadyKnown,
    #[error("`Transaction nonce too low`")]
    NonceTooLow,
    #[error("`Transaction fee too low`")]
    FeeTooLow,
    #[error("`bad origin")]
    BadOrigin,
    #[error("`insufficient funds for fee + amount`")]
    InsufficientFunds,
    #[error("`Explict coinbase transaction not allowed`")]
    ExplictCoinbase,
    #[error("`transaction in index missing from primary`")]
    TransactionNotFoundInPrimary,
    #[error("`{0}`")]
    HexError(#[from] hex::FromHexError),
}
