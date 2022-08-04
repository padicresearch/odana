// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The `prefix-hex` crates offers encoding and decoding of hex strings with a `0x` prefix.
//!
//! Its API aims to mimic that of the [`hex`](https://docs.rs/hex/latest/hex/) crate, which we also use internally.
//!
//! This crate is compatible with the hex encoding rules of the [Ethereum RPC API](https://eth.wiki/json-rpc/API#hex-value-encoding).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod data;
mod error;
#[cfg(feature = "primitive-types")]
mod primitive_types;

use alloc::string::String;

pub use error::Error;

/// Tries to decode an hexadecimal encoded string with a `0x` prefix.
pub trait FromHexPrefixed: Sized {
    /// Tries to decode an hexadecimal encoded string with a `0x` prefix.
    fn from_hex_prefixed(hex: &str) -> Result<Self, Error>;
}

/// Encodes data into an hexadecimal encoded string with a `0x` prefix.
pub trait ToHexPrefixed {
    /// Encodes data into an hexadecimal encoded string with a `0x` prefix.
    fn to_hex_prefixed(self) -> String;
}

/// Decodes a hex string with `0x` prefix into a type `T`.
///
/// ## Decode into `Vec<u8>`
/// ```
/// let result = prefix_hex::decode("0x000102");
/// assert_eq!(result, Ok(vec![0x0, 0x1, 0x2]));
/// ```
/// ## Decode into `[u8;N]`
/// ```
/// let result = prefix_hex::decode("0x000102");
/// assert_eq!(result, Ok([0x0, 0x1, 0x2]));
/// ```
pub fn decode<T: FromHexPrefixed>(hex: &str) -> Result<T, Error> {
    T::from_hex_prefixed(hex)
}

/// Encodes `T` as a hex string with a `0x` prefix.
///
/// ## Encode `Vec<u8>`
/// ```
/// assert_eq!(prefix_hex::encode(vec![0x1, 0x2, 0x3]), "0x010203");
/// ```
/// ## Encode `[u8; N]`
/// ```
/// assert_eq!(prefix_hex::encode([0x1, 0x2, 0x3]), "0x010203");
/// ```
pub fn encode<T: ToHexPrefixed>(value: T) -> String {
    ToHexPrefixed::to_hex_prefixed(value)
}

// TODO: Maybe introduce `handle_error` function with `#[cold]` attribute.
fn strip_prefix(hex: &str) -> Result<&str, Error> {
    if let Some(hex) = hex.strip_prefix("0x") {
        Ok(hex)
    } else if hex.len() < 2 {
        Err(Error::InvalidStringLength)
    } else {
        let mut chars = hex.chars();
        // Panic: the following two operations cannot panic because we checked for `hex.len()` in the `else if` branch.
        let c0 = chars.next().unwrap();
        let c1 = chars.next().unwrap();
        Err(Error::InvalidPrefix { c0, c1 })
    }
}
