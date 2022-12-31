use alloc::vec::Vec;
use core::fmt::Formatter;

#[derive(Debug)]
pub enum Error {
    KeyAlreadyEmpty,
    NonMembershipPathError(Vec<u8>, Vec<u8>),
    StorageError,
    StorageErrorKeyNotFound,
    BadProof(Vec<Vec<Vec<u8>>>),
}

impl core::fmt::Display  for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::KeyAlreadyEmpty => {
                writeln!(f, "KeyAlreadyEmpty")
            }
            Error::NonMembershipPathError(left, right) => {
                writeln!(f, "NonMembershipPathError {:?} {:?}", left, right )
            }
            Error::StorageError => {
                writeln!(f, "StorageError")
            }
            Error::StorageErrorKeyNotFound => {
                writeln!(f, "StorageErrorNotFound")
            }
            Error::BadProof(proof) => {
                writeln!(f, "BadProof {:?}", proof)
            }
        }
    }
}

impl core::error::Error for Error {}