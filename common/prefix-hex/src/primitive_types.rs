// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use alloc::{format, string::String};

use crate::{strip_prefix, Error, FromHexPrefixed, ToHexPrefixed};

macro_rules! impl_from_to_hex {
    ($type:ty) => {
        impl FromHexPrefixed for $type {
            fn from_hex_prefixed(hex: &str) -> Result<Self, Error> {
                let hex = strip_prefix(hex)?;

                if hex.is_empty() {
                    return Err(Error::InvalidStringLength);
                }

                <$type>::from_str_radix(hex, 16).map_err(|error| match error.kind() {
                    uint::FromStrRadixErrKind::InvalidCharacter => {
                        if let Some((index, c)) = hex.chars().enumerate().find(|(_, c)| !c.is_ascii_hexdigit()) {
                            Error::InvalidHexCharacter { c, index }
                        } else {
                            unreachable!()
                        }
                    }
                    uint::FromStrRadixErrKind::InvalidLength => Error::InvalidStringLength,
                    _ => unreachable!(),
                })
            }
        }

        impl ToHexPrefixed for $type {
            fn to_hex_prefixed(self) -> String {
                format!("{:#x}", self)
            }
        }
    };
}

impl_from_to_hex!(primitive_types::U128);
impl_from_to_hex!(primitive_types::U256);
impl_from_to_hex!(primitive_types::U512);
