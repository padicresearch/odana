use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("RWPoison")]
    RWPoison,
    #[error("ColumnFamilyMissing {0}")]
    ColumnFamilyMissing(&'static str),
}
