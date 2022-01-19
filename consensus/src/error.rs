use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("parent block not found")]
    ParentBlockNotFound,
}
