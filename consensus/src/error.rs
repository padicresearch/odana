use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("parent block not found")]
    ParentBlockNotFound,
    #[error("bad block target")]
    BlockBadTarget,
    #[error("bad proof of work")]
    BadPow,
}
