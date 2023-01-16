use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    EcdsaError(k256::ecdsa::Error),
    RSVInvalid,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::EcdsaError(t) => {
                writeln!(f, "EcdsaError {}", t)
            }
            Error::RSVInvalid => {
                writeln!(f, "RSVInvalid")
            }
        }
    }
}

impl From<k256::ecdsa::Error> for Error {
    fn from(value: k256::ecdsa::Error) -> Self {
        Error::EcdsaError(value)
    }
}

impl core::error::Error for Error {}