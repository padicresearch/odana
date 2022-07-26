use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Chain validation failed")]
    ChainValidationFailed,
}
