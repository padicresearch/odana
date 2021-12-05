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
    Ed25519Error(#[from] ed25519_dalek::ed25519::Error),
    #[error("`{0}`")]
    SerializationError(bincode::Error),
    #[error("`{0}`")]
    DeserializationError(bincode::Error),
    #[error("UTXOError `{0}`")]
    UTXOError(&'static str),
}