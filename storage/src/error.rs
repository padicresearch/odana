use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("RWPoison")]
    RWPoison,
}
