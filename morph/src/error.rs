use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("RWPoison")]
    RWPoison,
    #[error("`{0}`")]
    GenericError(#[from] anyhow::Error),
    #[error("`{0}`")]
    SerializationError(bincode::Error),
    #[error("`{0}`")]
    DeserializationError(bincode::Error),
    #[error("AccountNotFound")]
    AccountNotFound,
    #[error("ValidationFailed")]
    ValidationFailed,
    #[error("ValidationFailedHistoryNotFound")]
    ValidationFailedHistoryNotFound,
    #[error("ValidationFailedAccountState")]
    ValidationFailedAccountState,
    #[error("ValidationFailedRootNotValid")]
    ValidationFailedRootNotValid,
    #[error("TransactionFailed")]
    TransactionFailed,
}