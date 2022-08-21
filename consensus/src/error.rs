use thiserror::Error;
use primitive_types::Compact;

#[derive(Error, Debug)]
pub enum Error {
    #[error("parent block not found")]
    ParentBlockNotFound,
    #[error("bad block target")]
    BlockBadTarget,
    #[error("bad proof of work expected {0:?} got {1:?}")]
    BadPow(Compact,Compact),
}
