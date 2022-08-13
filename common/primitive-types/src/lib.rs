// Copyright 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Primitive types shared by Substrate and Parity Ethereum.
//!
//! Those are uint types `U128`, `U256` and `U512`, and fixed hash types `H160`,
//! `H256` and `H512`, with optional serde serialization, parity-scale-codec and
//! rlp encoding.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::reversed_empty_ranges)]
#![allow(clippy::assign_op_pattern)]

use core::convert::TryFrom;

#[cfg(feature = "scale-info")]
use scale_info_crate::TypeInfo;

use fixed_hash::{construct_fixed_hash, impl_fixed_hash_conversions};
use uint::{construct_uint, uint_full_mul_reg};

#[cfg(feature = "fp-conversion")]
mod fp_conversion;

/// Error type for conversion.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Overflow encountered.
    Overflow,
}

construct_uint! {
    /// 128-bit unsigned integer.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct U128(2);
}
construct_uint! {
    /// 256-bit unsigned integer.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct U192(3);
}

construct_uint! {
    /// 256-bit unsigned integer.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct U256(4);
}
construct_uint! {
    /// 512-bits unsigned integer.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct U512(8);
}

construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 16 bytes (128 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H128(16);
}

construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 20 bytes (160 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H160(20);
}
construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 32 bytes (256 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H192(24);
}

construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 32 bytes (256 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H256(32);
}

construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 64 bytes (512 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H448(56);
}

construct_fixed_hash! {
    /// Fixed-size uninterpreted hash type with 64 bytes (512 bits) size.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct H512(64);
}

#[cfg(feature = "num-traits")]
mod num_traits {
    use impl_num_traits::impl_uint_num_traits;

    use super::*;

    impl_uint_num_traits!(U128, 2);
    impl_uint_num_traits!(U192, 3);
    impl_uint_num_traits!(U256, 4);
    impl_uint_num_traits!(U512, 8);
}

#[cfg(feature = "impl-serde")]
mod serde {
    use impl_serde::{impl_fixed_hash_serde, impl_uint_serde};

    use super::*;

    impl_uint_serde!(U128, 2);
    impl_uint_serde!(U192, 3);
    impl_uint_serde!(U256, 4);
    impl_uint_serde!(U512, 8);

    impl_fixed_hash_serde!(H128, 16);
    impl_fixed_hash_serde!(H160, 20);
    impl_fixed_hash_serde!(H192, 24);
    impl_fixed_hash_serde!(H256, 32);
    impl_fixed_hash_serde!(H448, 56);
    impl_fixed_hash_serde!(H512, 64);
    impl_fixed_hash_conversions!(H256, H160);
}

#[cfg(feature = "impl-codec")]
mod codec {
    use impl_codec::{impl_fixed_hash_codec, impl_uint_codec};

    use super::*;

    impl_uint_codec!(U128, 2);
    impl_uint_codec!(U256, 4);
    impl_uint_codec!(U512, 8);

    impl_fixed_hash_codec!(H128, 16);
    impl_fixed_hash_codec!(H160, 20);
    impl_fixed_hash_codec!(H256, 32);
    impl_fixed_hash_codec!(H512, 64);
}

#[cfg(feature = "impl-rlp")]
mod rlp {
    use impl_rlp::{impl_fixed_hash_rlp, impl_uint_rlp};

    use super::*;

    impl_uint_rlp!(U128, 2);
    impl_uint_rlp!(U192, 3);
    impl_uint_rlp!(U256, 4);
    impl_uint_rlp!(U512, 8);

    impl_fixed_hash_rlp!(H128, 16);
    impl_fixed_hash_rlp!(H160, 20);
    impl_fixed_hash_rlp!(H192, 24);
    impl_fixed_hash_rlp!(H256, 32);
    impl_fixed_hash_rlp!(H448, 56);
    impl_fixed_hash_rlp!(H512, 64);
}

impl U128 {
    /// Multiplies two 128-bit integers to produce full 256-bit integer.
    /// Overflow is not possible.
    #[inline(always)]
    pub fn full_mul(self, other: U128) -> U256 {
        U256(uint_full_mul_reg!(U128, 2, self, other))
    }
    #[inline(always)]
    pub fn to_be_bytes(self) -> [u8; 16] {
        let mut out = [0_u8; 16];
        self.to_big_endian(&mut out);
        out
    }
}

impl U192 {
    #[inline(always)]
    pub fn to_be_bytes(self) -> [u8; 24] {
        let mut out = [0_u8; 24];
        self.to_big_endian(&mut out);
        out
    }

    #[inline(always)]
    pub fn to_le_bytes(self) -> [u8; 24] {
        let mut out = [0_u8; 24];
        self.to_little_endian(&mut out);
        out
    }
}

impl U256 {
    /// Multiplies two 256-bit integers to produce full 512-bit integer.
    /// Overflow is not possible.
    #[inline(always)]
    pub fn full_mul(self, other: U256) -> U512 {
        U512(uint_full_mul_reg!(U256, 4, self, other))
    }
    #[inline(always)]
    pub fn to_be_bytes(self) -> [u8; 32] {
        let mut out = [0_u8; 32];
        self.to_big_endian(&mut out);
        out
    }
}

impl From<U256> for U512 {
    fn from(value: U256) -> U512 {
        let U256(ref arr) = value;
        let mut ret = [0; 8];
        ret[0] = arr[0];
        ret[1] = arr[1];
        ret[2] = arr[2];
        ret[3] = arr[3];
        U512(ret)
    }
}

impl TryFrom<U256> for U128 {
    type Error = Error;

    fn try_from(value: U256) -> Result<U128, Error> {
        let U256(ref arr) = value;
        if arr[2] | arr[3] != 0 {
            return Err(Error::Overflow);
        }
        let mut ret = [0; 2];
        ret[0] = arr[0];
        ret[1] = arr[1];
        Ok(U128(ret))
    }
}

impl TryFrom<U512> for U256 {
    type Error = Error;

    fn try_from(value: U512) -> Result<U256, Error> {
        let U512(ref arr) = value;
        if arr[4] | arr[5] | arr[6] | arr[7] != 0 {
            return Err(Error::Overflow);
        }
        let mut ret = [0; 4];
        ret[0] = arr[0];
        ret[1] = arr[1];
        ret[2] = arr[2];
        ret[3] = arr[3];
        Ok(U256(ret))
    }
}

impl TryFrom<U512> for U128 {
    type Error = Error;

    fn try_from(value: U512) -> Result<U128, Error> {
        let U512(ref arr) = value;
        if arr[2] | arr[3] | arr[4] | arr[5] | arr[6] | arr[7] != 0 {
            return Err(Error::Overflow);
        }
        let mut ret = [0; 2];
        ret[0] = arr[0];
        ret[1] = arr[1];
        Ok(U128(ret))
    }
}

impl From<U128> for U512 {
    fn from(value: U128) -> U512 {
        let U128(ref arr) = value;
        let mut ret = [0; 8];
        ret[0] = arr[0];
        ret[1] = arr[1];
        U512(ret)
    }
}

impl From<U128> for U256 {
    fn from(value: U128) -> U256 {
        let U128(ref arr) = value;
        let mut ret = [0; 4];
        ret[0] = arr[0];
        ret[1] = arr[1];
        U256(ret)
    }
}

impl<'a> From<&'a U256> for U512 {
    fn from(value: &'a U256) -> U512 {
        let U256(ref arr) = *value;
        let mut ret = [0; 8];
        ret[0] = arr[0];
        ret[1] = arr[1];
        ret[2] = arr[2];
        ret[3] = arr[3];
        U512(ret)
    }
}

impl<'a> TryFrom<&'a U512> for U256 {
    type Error = Error;

    fn try_from(value: &'a U512) -> Result<U256, Error> {
        let U512(ref arr) = *value;
        if arr[4] | arr[5] | arr[6] | arr[7] != 0 {
            return Err(Error::Overflow);
        }
        let mut ret = [0; 4];
        ret[0] = arr[0];
        ret[1] = arr[1];
        ret[2] = arr[2];
        ret[3] = arr[3];
        Ok(U256(ret))
    }
}

/// Compact representation of `U256`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Compact(u32);

impl From<u32> for Compact {
    fn from(u: u32) -> Self {
        Compact(u)
    }
}

impl From<Compact> for u32 {
    fn from(c: Compact) -> Self {
        c.0
    }
}

impl From<U256> for Compact {
    fn from(u: U256) -> Self {
        Compact::from_u256(u)
    }
}

impl From<Compact> for U256 {
    fn from(c: Compact) -> Self {
        // ignore overflows and negative values
        c.to_u256().unwrap_or_else(|x| x)
    }
}

impl Compact {
    pub fn new(u: u32) -> Self {
        Compact(u)
    }

    pub fn max_value() -> Self {
        U256::max_value().into()
    }

    /// Computes the target [0, T] that a blockhash must land in to be valid
    /// Returns value in error, if there is an overflow or its negative value
    pub fn to_u256(&self) -> Result<U256, U256> {
        let size = self.0 >> 24;
        let mut word = self.0 & 0x007fffff;

        let result = if size <= 3 {
            word >>= 8 * (3 - size as usize);
            word.into()
        } else {
            U256::from(word) << (8 * (size as usize - 3))
        };

        let is_negative = word != 0 && (self.0 & 0x00800000) != 0;
        let is_overflow =
            (word != 0 && size > 34) || (word > 0xff && size > 33) || (word > 0xffff && size > 32);

        if is_negative || is_overflow {
            Err(result)
        } else {
            Ok(result)
        }
    }

    pub fn from_u256(val: U256) -> Self {
        let mut size = (val.bits() + 7) / 8;
        let mut compact = if size <= 3 {
            (val.low_u64() << (8 * (3 - size))) as u32
        } else {
            let bn = val >> (8 * (size - 3));
            bn.low_u32()
        };

        if (compact & 0x00800000) != 0 {
            compact >>= 8;
            size += 1;
        }

        assert!((compact & !0x007fffff) == 0);
        assert!(size < 256);
        Compact(compact | (size << 24) as u32)
    }

    pub fn to_f64(&self) -> f64 {
        let mut shift = (self.0 >> 24) & 0xff;
        let mut diff = f64::from(0x0000ffffu32) / f64::from(self.0 & 0x00ffffffu32);
        while shift < 29 {
            diff *= f64::from(256);
            shift += 1;
        }
        while shift > 29 {
            diff /= 256.0;
            shift -= 1;
        }
        diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_to_u256() {
        assert_eq!(Compact::new(0x01003456).to_u256(), Ok(0.into()));
        assert_eq!(Compact::new(0x01123456).to_u256(), Ok(0x12.into()));
        assert_eq!(Compact::new(0x02008000).to_u256(), Ok(0x80.into()));
        assert_eq!(Compact::new(0x05009234).to_u256(), Ok(0x92340000u64.into()));
        // negative -0x12345600
        assert!(Compact::new(0x04923456).to_u256().is_err());
        assert_eq!(Compact::new(0x04123456).to_u256(), Ok(0x12345600u64.into()));
    }

    #[test]
    fn test_from_u256() {
        let test1 = U256::from(1000u64);
        assert_eq!(Compact::new(0x0203e800), Compact::from_u256(test1));

        let test2 = U256::from(2).pow(U256::from(256 - 32)) - U256::from(1);
        assert_eq!(Compact::new(0x1d00ffff), Compact::from_u256(test2));
    }

    #[test]
    fn test_compact_to_from_u256() {
        // TODO: it does not work both ways for small values... check why
        let compact = Compact::new(0x1d00ffff);
        let compact2 = Compact::from_u256(compact.to_u256().unwrap());
        assert_eq!(compact, compact2);

        let compact = Compact::new(0x05009234);
        let compact2 = Compact::from_u256(compact.to_u256().unwrap());
        assert_eq!(compact, compact2);
    }

    #[test]
    fn difficulty() {
        fn compare_f64(v1: f64, v2: f64) -> bool {
            (v1 - v2).abs() < 0.00001
        }

        assert!(compare_f64(Compact::new(0x1b0404cb).to_f64(), 16307.42094));

        // tests from original bitcoin client:
        // https://github.com/bitcoin/bitcoin/blob/1e8f88e071019907785b260477bd359bef6f9a8f/src/test/blockchain_tests.cpp

        assert!(compare_f64(Compact::new(0x1f111111).to_f64(), 0.000001));
        assert!(compare_f64(Compact::new(0x1ef88f6f).to_f64(), 0.000016));
        assert!(compare_f64(Compact::new(0x1df88f6f).to_f64(), 0.004023));
        assert!(compare_f64(Compact::new(0x1cf88f6f).to_f64(), 1.029916));
        assert!(compare_f64(
            Compact::new(0x12345678).to_f64(),
            5913134931067755359633408.0,
        ));
    }

    #[test]
    fn print_difficulty() {
        pub const NODE_POW_TARGET: U256 = U256([
            0x0000000000000000u64,
            0x0000000000000000u64,
            0x0000000000000000u64,
            0x00000fffff000000u64,
        ]);
        let pow = U256::from_f64_lossy(26.0);
        let t = make_target(26.0);
        let c = Compact::from_u256(NODE_POW_TARGET);
        let tc = Compact::from_u256(t);
        println!("Compact {}", c.to_f64());
        println!("Compact {}", pow.as_u32());
        println!("Compact P {:#?}", pow.0);
        println!("Compact J {:#?}", NODE_POW_TARGET.0);
        println!("Compact T {}", t);
        println!("Compact T {:#?}", t.0);
        println!("Compact T {}", tc.0);
        println!("Compact T {}", tc.to_f64());
        println!("Compact {}", c.0);
    }
}
