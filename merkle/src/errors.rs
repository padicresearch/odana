use thiserror::Error;

#[derive(Error, Debug)]
pub enum MerkleError {
    #[error("hash of item already exist")]
    MerkleTreeUpdateError,
    #[error("hash function error")]
    HashFunctionError,
    #[error("unknown merkle error")]
    Unknown,
}
