use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("RWPoison")]
    RWPoison,
    #[error("`{0}`")]
    GenericError(#[from] anyhow::Error),
    #[error("`CommitLogMessageError {0:?}`")]
    CommitLogMessageError(commitlog::message::MessageError),
    #[error("`CommitLogReadError {0}`")]
    CommitLogReadError(#[from] commitlog::ReadError),
    #[error("`CommitLogReadErrorCorruptData`")]
    CommitLogReadErrorCorruptData,
    #[error("AccountNotFound")]
    AccountNotFound,
    #[error("ValidationFailed")]
    ValidationFailed,
    #[error("TransactionAlreadyApplied")]
    TransactionAlreadyApplied,
    #[error("ValidationFailedHistoryNotFound")]
    ValidationFailedHistoryNotFound,
    #[error("ValidationFailedAccountState")]
    ValidationFailedAccountState,
    #[error("ValidationFailedRootNotValid")]
    ValidationFailedRootNotValid,
    #[error("TransactionFailed")]
    TransactionFailed,
    #[error("SnapshotCreationErrorRootNotFound")]
    SnapshotCreationErrorRootNotFound,
    #[error("GenesisAlreadyInitialized")]
    GenesisAlreadyInitialized,
    #[error("NonceIsLessThanCurrent")]
    NonceIsLessThanCurrent,
    #[error("LogIndexNoFound")]
    LogIndexNoFound,
}