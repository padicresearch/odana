use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("RWPoison")]
    RWPoison,
    #[error("ColumnFamilyMissing {0}")]
    ColumnFamilyMissing(&'static str),

    #[error("Invalid Key {0}")]
    InvalidKey(String),

    #[error("KeyAlreadyEmpty")]
    KeyAlreadyEmpty,
    #[error("Non member {0} is equal to member {1}")]
    NonMembershipPathError(String, String),

    #[error("BadProof")]
    BadProof(Vec<Vec<Vec<u8>>>),
}
