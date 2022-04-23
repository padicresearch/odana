use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("RWPoison")]
    RWPoison,
    #[error("ColumnFamilyMissing {0}")]
    ColumnFamilyMissing(&'static str),

    #[error("Invalid Key {0:#?}")]
    InvalidKey(Vec<u8>),

    #[error("KeyAlreadyEmpty")]
    KeyAlreadyEmpty,
}