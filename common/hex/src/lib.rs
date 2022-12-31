#![no_std]

pub mod serde;

extern crate core;
extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

static CHARS: &[u8] = b"0123456789abcdef";

pub use crate::serde::{deserialize, serialize};

pub trait ToHex {
    /// Encode the hex strict representing `self` into the result. Lower case
    /// letters are used (e.g. `f9b4ca`)
    fn encode_hex(&self) -> String;
}
pub trait FromHex: Sized {
    /// Creates an instance of type `Self` from the given hex string, or fails
    /// with a custom error type.
    ///
    /// Both, upper and lower case characters are valid and can even be
    /// mixed (e.g. `f9b4ca`, `F9B4CA` and `f9B4Ca` are all valid strings).
    fn from_hex(hex: &str) -> Result<Self, FromHexError>;
}

impl ToHex for Vec<u8> {
    fn encode_hex(&self) -> String {
        encode(self, true)
    }
}
impl FromHex for Vec<u8> {
    fn from_hex(hex: &str) -> Result<Self, FromHexError> {
        decode(hex)
    }
}

pub trait HexEncodeLen {
    fn encoded_hex_len(&self) -> usize;
}
/// Serialize given bytes to a 0x-prefixed hex string.
///
/// If `skip_leading_zero` initial 0s will not be printed out,
/// unless the byte string is empty, in which case `0x0` will be returned.
/// The results are consistent with `serialize_uint` output if the flag is
/// on and `serialize_raw` if the flag is off.
pub fn encode<B: AsRef<[u8]>>(bytes: B, skip_leading_zero: bool) -> String {
    let bytes = bytes.as_ref();
    let bytes = if skip_leading_zero {
        let non_zero = bytes.iter().take_while(|b| **b == 0).count();
        let bytes = &bytes[non_zero..];
        if bytes.is_empty() {
            return "0x0".into();
        } else {
            bytes
        }
    } else if bytes.is_empty() {
        return "0x".into();
    } else {
        bytes
    };

    let mut slice = vec![0u8; (bytes.len() + 1) * 2];
    encode_to_slice(&mut slice, bytes, skip_leading_zero).into()
}

pub fn encode_to_slice<'a>(v: &'a mut [u8], bytes: &[u8], skip_leading_zero: bool) -> &'a str {
    assert!(v.len() > 1 + bytes.len() * 2);

    v[0] = b'0';
    v[1] = b'x';

    let mut idx = 2;
    let first_nibble = bytes[0] >> 4;
    if first_nibble != 0 || !skip_leading_zero {
        v[idx] = CHARS[first_nibble as usize];
        idx += 1;
    }
    v[idx] = CHARS[(bytes[0] & 0xf) as usize];
    idx += 1;

    for &byte in bytes.iter().skip(1) {
        v[idx] = CHARS[(byte >> 4) as usize];
        v[idx + 1] = CHARS[(byte & 0xf) as usize];
        idx += 2;
    }

    // SAFETY: all characters come either from CHARS or "0x", therefore valid UTF8
    unsafe { core::str::from_utf8_unchecked(&v[0..idx]) }
}

/// Decoding bytes from hex string error.
#[derive(Debug, PartialEq, Eq)]
pub enum FromHexError {
    /// The `0x` prefix is missing.
    #[deprecated(since = "0.3.2", note = "We support non 0x-prefixed hex strings")]
    MissingPrefix,

    InvalidLength,
    /// Invalid (non-hex) character encountered.
    InvalidHex {
        /// The unexpected character.
        character: char,
        /// Index of that occurrence.
        index: usize,
    },
}

#[cfg(feature = "std")]
impl std::error::Error for FromHexError {}

impl fmt::Display for FromHexError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            #[allow(deprecated)]
            Self::MissingPrefix => write!(fmt, "0x prefix is missing"),
            Self::InvalidHex { character, index } => {
                write!(fmt, "invalid hex character: {}, at {}", character, index)
            }
            FromHexError::InvalidLength => write!(fmt, "invalid length"),
        }
    }
}

/// Decode given (both 0x-prefixed or not) hex string into a vector of bytes.
///
/// Returns an error if non-hex characters are present.
pub fn decode(v: &str) -> Result<Vec<u8>, FromHexError> {
    let (v, stripped) = v.strip_prefix("0x").map_or((v, false), |v| (v, true));
    let mut bytes = vec![0u8; (v.len() + 1) / 2];
    decode_to_slice(v, &mut bytes, stripped)?;
    Ok(bytes)
}

/// Decode given 0x-prefix-stripped hex string into provided slice.
/// Used internally by `from_hex` and `deserialize_check_len`.
///
/// The method will panic if `bytes` have incorrect length (make sure to allocate enough beforehand).
pub fn decode_to_slice(v: &str, bytes: &mut [u8], stripped: bool) -> Result<usize, FromHexError> {
    let bytes_len = v.len();
    let mut modulus = bytes_len % 2;
    let mut buf = 0;
    let mut pos = 0;
    for (index, byte) in v.bytes().enumerate() {
        buf <<= 4;

        match byte {
            b'A'..=b'F' => buf |= byte - b'A' + 10,
            b'a'..=b'f' => buf |= byte - b'a' + 10,
            b'0'..=b'9' => buf |= byte - b'0',
            b' ' | b'\r' | b'\n' | b'\t' => {
                buf >>= 4;
                continue;
            }
            b => {
                let character = char::from(b);
                return Err(FromHexError::InvalidHex {
                    character,
                    index: index + if stripped { 2 } else { 0 },
                });
            }
        }

        modulus += 1;
        if modulus == 2 {
            modulus = 0;
            bytes[pos] = buf;
            pos += 1;
        }
    }

    Ok(pos)
}

/// Hex encode a slice of bytes.
pub fn encode_raw(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        String::from("0x")
    } else {
        encode(bytes, false)
    }
}

/// Hex encode a slice of bytes as uint.
///
/// The representation will have all leading zeros trimmed.
pub fn encode_uint(bytes: &[u8]) -> String {
    let non_zero = bytes.iter().take_while(|b| **b == 0).count();
    let bytes = &bytes[non_zero..];
    if bytes.is_empty() {
        String::from("0x0")
    } else {
        encode(bytes, true)
    }
}

macro_rules! impl_hex_uint {
    ($name: ident, $len: expr) => {
        impl ToHex for $name {
            fn encode_hex(&self) -> String {
                let bytes = self.to_be_bytes();
                encode_uint(&bytes)
            }
        }

        impl FromHex for $name {
            fn from_hex(v: &str) -> Result<$name, FromHexError> {
                let mut raw = [0; $len];
                let decoded = decode(v)?;
                let start_index = $len - decoded.len();
                let mut iter = decoded.iter().copied();
                for i in start_index..$len {
                    raw[i] = iter.next().ok_or(FromHexError::InvalidLength)?;
                }
                Ok($name::from_be_bytes(raw))
            }
        }
    };
}

impl_hex_uint!(u8, 1);
impl_hex_uint!(u16, 2);
impl_hex_uint!(u32, 4);
impl_hex_uint!(u64, 8);
impl_hex_uint!(u128, 16);

macro_rules! from_hex_array_impl {
    ($($len:expr)+) => {$(
        impl ToHex for [u8; $len] {
            fn encode_hex(&self) -> String {
                encode(&self, false)
            }
        }
        impl FromHex for [u8; $len] {
            fn from_hex(hex: &str) -> Result<Self, FromHexError> {
                let mut out = [0_u8; $len];
                decode_to_slice(hex, &mut out as &mut [u8], false)?;
                Ok(out)
            }
        }
    )+}
}

from_hex_array_impl! {
    1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16
    17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32
    33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48
    49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64
    65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80
    81 82 83 84 85 86 87 88 89 90 91 92 93 94 95 96
    97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112
    113 114 115 116 117 118 119 120 121 122 123 124 125 126 127 128
    160 192 200 224 256 384 512 768 1024 2048 4096 8192 16384 32768
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use super::*;

    #[test]
    fn should_encode_to_and_from_hex_with_prefix() {
        assert_eq!(encode(&[0, 1, 2], true), "0x102");
        assert_eq!([0, 1, 2].encode_hex(), "0x102");
        assert_eq!(encode(&[0, 1, 2], false), "0x000102");
        assert_eq!(encode(&[0], true), "0x0");
        assert_eq!(encode(&[], true), "0x0");
        assert_eq!(encode(&[], false), "0x");
        assert_eq!(encode(&[0], false), "0x00");
        assert_eq!(decode("0x0102"), Ok(vec![1, 2]));
        assert_eq!(decode("0x102"), Ok(vec![1, 2]));
        assert_eq!(decode("0xf"), Ok(vec![0xf]));
    }

    #[test]
    fn should_decode_hex_without_prefix() {
        assert_eq!(decode("0102"), Ok(vec![1, 2]));
        assert_eq!(decode("102"), Ok(vec![1, 2]));
        assert_eq!(decode("f"), Ok(vec![0xf]));
    }

    #[test]
    fn should_encode_from_primitives() {
        assert_eq!(u8::from_hex(&20_u8.encode_hex()).unwrap(), 20);
        assert_eq!(u16::from_hex(&300_u16.encode_hex()).unwrap(), 300);
        assert_eq!(u16::from_hex(&1_u16.encode_hex()).unwrap(), 1);
        assert_eq!(
            u128::from_hex(&3000000000000_u128.encode_hex()).unwrap(),
            3000000000000
        );
        assert_eq!(u32::from_hex(&u32::MAX.encode_hex()).unwrap(), u32::MAX);
        assert_eq!(u64::from_hex(&u64::MAX.encode_hex()).unwrap(), u64::MAX);
        assert_eq!(u128::from_hex(&u128::MAX.encode_hex()).unwrap(), u128::MAX);
        assert_eq!(u128::from_hex(&1_u128.encode_hex()).unwrap(), 1);
        assert_eq!(u128::from_hex(&0_u128.encode_hex()).unwrap(), 0);
    }
}
