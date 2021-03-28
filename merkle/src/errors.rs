use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct HashFunctionError;

impl Error for HashFunctionError {}

impl Display for HashFunctionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Debug)]
pub struct MerkleTreeUpdateError;

impl Error for MerkleTreeUpdateError {}

impl Display for MerkleTreeUpdateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "hash of item already exist")
    }
}

