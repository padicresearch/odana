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
mod fp_conversion;
use core::convert::TryFrom;
use fixed_hash::{construct_fixed_hash, impl_fixed_hash_conversions};
use uint::{construct_uint, uint_full_mul_reg};
use impl_num_traits::impl_uint_num_traits;
use impl_serde::{impl_fixed_hash_serde, impl_uint_serde};
/// Error type for conversion.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// Overflow encountered.
	Overflow,
}

construct_uint! {
	pub struct U128(2);
}
construct_uint! {
	pub struct U256(4);
}
construct_uint! {
	pub struct U512(8);
}

construct_fixed_hash! {
	pub struct H128(16);
}

construct_fixed_hash! {
	pub struct H160(20);
}
construct_fixed_hash! {
	pub struct H256(32);
}
construct_fixed_hash! {
	pub struct H512(64);
}



impl_uint_num_traits!(U128, 2);
impl_uint_num_traits!(U256, 4);
impl_uint_num_traits!(U512, 8);



impl_uint_serde!(U128, 2);
impl_uint_serde!(U256, 4);
impl_uint_serde!(U512, 8);

impl_fixed_hash_serde!(H128, 16);
impl_fixed_hash_serde!(H160, 20);
impl_fixed_hash_serde!(H256, 32);
impl_fixed_hash_serde!(H512, 64);
impl_fixed_hash_conversions!(H256, H160);

impl U128 {
	/// Multiplies two 128-bit integers to produce full 256-bit integer.
	/// Overflow is not possible.
	#[inline(always)]
	pub fn full_mul(self, other: U128) -> U256 {
		U256(uint_full_mul_reg!(U128, 2, self, other))
	}
}

impl U256 {
	/// Multiplies two 256-bit integers to produce full 512-bit integer.
	/// Overflow is not possible.
	#[inline(always)]
	pub fn full_mul(self, other: U256) -> U512 {
		U512(uint_full_mul_reg!(U256, 4, self, other))
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
			return Err(Error::Overflow)
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
			return Err(Error::Overflow)
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
			return Err(Error::Overflow)
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
			return Err(Error::Overflow)
		}
		let mut ret = [0; 4];
		ret[0] = arr[0];
		ret[1] = arr[1];
		ret[2] = arr[2];
		ret[3] = arr[3];
		Ok(U256(ret))
	}
}
