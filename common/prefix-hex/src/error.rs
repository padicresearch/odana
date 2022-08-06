// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

use hex::FromHexError;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPrefix { c0: char, c1: char },
    InvalidHexCharacter { c: char, index: usize },
    InvalidStringLength,
    InvalidStringLengthSlice { expected: usize, actual: usize },
    OddLength,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<FromHexError> for Error {
    fn from(v: FromHexError) -> Error {
        match v {
            FromHexError::InvalidHexCharacter { c, index } => Error::InvalidHexCharacter { c, index },
            FromHexError::InvalidStringLength => Error::InvalidStringLength,
            FromHexError::OddLength => Error::OddLength,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidPrefix { c0, c1 } => {
                write!(f, "Invalid hex prefix: expected `0x` but got `{c0}{c1}`")
            }
            Error::InvalidHexCharacter { c, index } => {
                write!(f, "Invalid hex character {:?} at position {}", c, index)
            }
            Error::InvalidStringLength => write!(f, "Invalid hex string length"),
            Error::InvalidStringLengthSlice { expected, actual } => write!(
                f,
                "Invalid hex string length for slice: expected {expected} got {actual}"
            ),
            Error::OddLength => write!(f, "Odd number of digits in hex string"),
        }
    }
}
