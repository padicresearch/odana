// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::{string::String, vec::Vec};
use core::{fmt, result::Result};

use serde::{de, Deserializer, Serializer};

/// Serializes a slice of bytes.
pub fn serialize_raw<S>(slice: &mut [u8], bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if bytes.is_empty() {
        serializer.serialize_str("0x")
    } else {
        serializer.serialize_str(hex::encode_to_slice(slice, bytes, false))
    }
}

/// Serializes a slice of bytes.
pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut slice = vec![0u8; (bytes.len() + 1) * 2];
    serialize_raw(&mut slice, bytes, serializer)
}

/// Serialize a slice of bytes as uint.
///
/// The representation will have all leading zeros trimmed.
pub fn serialize_uint<S>(slice: &mut [u8], bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let non_zero = bytes.iter().take_while(|b| **b == 0).count();
    let bytes = &bytes[non_zero..];
    if bytes.is_empty() {
        serializer.serialize_str("0x0")
    } else {
        serializer.serialize_str(hex::encode_to_slice(slice, bytes, true))
    }
}

/// Expected length of bytes vector.
#[derive(Debug, PartialEq, Eq)]
pub enum ExpectedLen<'a> {
    /// Exact length in bytes.
    Exact(&'a mut [u8]),
    /// A bytes length between (min; slice.len()].
    Between(usize, &'a mut [u8]),
}

impl<'a> fmt::Display for ExpectedLen<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExpectedLen::Exact(ref v) => write!(fmt, "length of {}", v.len() * 2),
            ExpectedLen::Between(min, ref v) => {
                write!(fmt, "length between ({}; {}]", min * 2, v.len() * 2)
            }
        }
    }
}

/// Deserialize into vector of bytes.  This will allocate an O(n) intermediate
/// string.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'b> de::Visitor<'b> for Visitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a (both 0x-prefixed or not) hex string")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            hex::decode(v).map_err(E::custom)
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            self.visit_str(&v)
        }
    }

    deserializer.deserialize_str(Visitor)
}

/// Deserialize into vector of bytes with additional size check.
/// Returns number of bytes written.
pub fn deserialize_check_len<'a, 'de, D>(
    deserializer: D,
    len: ExpectedLen<'a>,
) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor<'a> {
        len: ExpectedLen<'a>,
    }

    impl<'a, 'b> de::Visitor<'b> for Visitor<'a> {
        type Value = usize;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(
                formatter,
                "a (both 0x-prefixed or not) hex string with {}",
                self.len
            )
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let (v, stripped) = v.strip_prefix("0x").map_or((v, false), |v| (v, true));

            let len = v.len();
            let is_len_valid = match self.len {
                ExpectedLen::Exact(ref slice) => len == 2 * slice.len(),
                ExpectedLen::Between(min, ref slice) => len <= 2 * slice.len() && len > 2 * min,
            };

            if !is_len_valid {
                return Err(E::invalid_length(v.len(), &self));
            }

            let bytes = match self.len {
                ExpectedLen::Exact(slice) => slice,
                ExpectedLen::Between(_, slice) => slice,
            };

            hex::decode_to_slice(v.as_ref(), bytes, stripped).map_err(E::custom)
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            self.visit_str(&v)
        }
    }

    deserializer.deserialize_str(Visitor { len })
}

#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Bytes(#[serde(with = "super")] Vec<u8>);

    #[test]
    fn should_not_fail_on_short_string_with_prefix() {
        let a: Bytes = serde_json::from_str("\"0x\"").unwrap();
        let b: Bytes = serde_json::from_str("\"0x1\"").unwrap();
        let c: Bytes = serde_json::from_str("\"0x12\"").unwrap();
        let d: Bytes = serde_json::from_str("\"0x123\"").unwrap();
        let e: Bytes = serde_json::from_str("\"0x1234\"").unwrap();
        let f: Bytes = serde_json::from_str("\"0x12345\"").unwrap();

        assert!(a.0.is_empty());
        assert_eq!(b.0, vec![1]);
        assert_eq!(c.0, vec![0x12]);
        assert_eq!(d.0, vec![0x1, 0x23]);
        assert_eq!(e.0, vec![0x12, 0x34]);
        assert_eq!(f.0, vec![0x1, 0x23, 0x45]);
    }

    #[test]
    fn should_not_fail_on_other_strings_with_prefix() {
        let a: Bytes = serde_json::from_str(
            "\"0x7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587\"",
        )
        .unwrap();
        let b: Bytes = serde_json::from_str(
            "\"0x7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587b\"",
        )
        .unwrap();
        let c: Bytes = serde_json::from_str(
            "\"0x7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587b4\"",
        )
        .unwrap();

        assert_eq!(a.0.len(), 31);
        assert_eq!(b.0.len(), 32);
        assert_eq!(c.0.len(), 32);
    }

    #[test]
    fn should_not_fail_on_short_string_without_prefix() {
        let a: Bytes = serde_json::from_str("\"\"").unwrap();
        let b: Bytes = serde_json::from_str("\"1\"").unwrap();
        let c: Bytes = serde_json::from_str("\"12\"").unwrap();
        let d: Bytes = serde_json::from_str("\"123\"").unwrap();
        let e: Bytes = serde_json::from_str("\"1234\"").unwrap();
        let f: Bytes = serde_json::from_str("\"12345\"").unwrap();

        assert!(a.0.is_empty());
        assert_eq!(b.0, vec![1]);
        assert_eq!(c.0, vec![0x12]);
        assert_eq!(d.0, vec![0x1, 0x23]);
        assert_eq!(e.0, vec![0x12, 0x34]);
        assert_eq!(f.0, vec![0x1, 0x23, 0x45]);
    }

    #[test]
    fn should_not_fail_on_other_strings_without_prefix() {
        let a: Bytes = serde_json::from_str(
            "\"7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587\"",
        )
        .unwrap();
        let b: Bytes = serde_json::from_str(
            "\"7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587b\"",
        )
        .unwrap();
        let c: Bytes = serde_json::from_str(
            "\"7f864e18e3dd8b58386310d2fe0919eef27c6e558564b7f67f22d99d20f587b4\"",
        )
        .unwrap();

        assert_eq!(a.0.len(), 31);
        assert_eq!(b.0.len(), 32);
        assert_eq!(c.0.len(), 32);
    }

    #[test]
    fn should_serialize_and_deserialize_empty_bytes() {
        let bytes = Bytes(Vec::new());

        let data = serde_json::to_string(&bytes).unwrap();

        assert_eq!("\"0x\"", &data);

        let deserialized: Bytes = serde_json::from_str(&data).unwrap();
        assert!(deserialized.0.is_empty())
    }
}
