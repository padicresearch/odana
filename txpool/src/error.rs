use thiserror::Error;

#[derive(Error, Debug)]
pub enum TxPoolError {
    #[error("RWPoison")]
    RWPoison,
    #[error("MutexGuard error {0}")]
    MutexGuardError(String),
    #[error("MutexGuard error {0:#?}")]
    CompositeErrors(Vec<String>),
    #[error("`{0}`")]
    GenericError(#[from] anyhow::Error),
    #[error("`Transaction is already known`")]
    TransactionAlreadyKnown,
    #[error("`Transaction nonce too low`")]
    NonceTooLow,
    #[error("`Transaction fee too low`")]
    FeeTooLow,
    #[error("`bad origin")]
    BadOrigin,
    #[error("`insufficient funds for fee: {0} + amount: {1}`")]
    InsufficientFunds(u128, u128),
    #[error("`Explict coinbase transaction not allowed`")]
    ExplictCoinbase,
    #[error("`transaction in index missing from primary`")]
    TransactionNotFoundInPrimary,
    #[error("`transaction is underpriced`")]
    Underpriced,
    #[error("`transaction pool overflow`")]
    TxPoolOverflow,
    #[error("`replacement transaction underpriced`")]
    ReplaceUnderpriced,
    #[error("`missing block`")]
    MissingBlock,
    #[error("`{0}`")]
    HexError(#[from] hex::FromHexError),
}
