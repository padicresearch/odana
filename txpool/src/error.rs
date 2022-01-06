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
    #[error("`TransactionAlreadyKnown`")]
    TransactionAlreadyKnown,
    #[error("`transaction in index missing from primary`")]
    TransactionNotFoundInPrimary,
    #[error("`{0}`")]
    HexError(#[from] hex::FromHexError),
}
