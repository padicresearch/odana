use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    EcdsaError(#[from] k256::ecdsa::Error),
    #[error("`{0}`")]
    InternalError(String),
}
