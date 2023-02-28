/*
 * Copyright (c) 2023 Padic Research.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::{Error, ADDRESS_LEN, H160};
use core::fmt::{Debug, Display, Formatter};
use core::str::FromStr;

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct Address(pub [u8; 44]);

impl Default for Address {
    fn default() -> Self {
        Self([0; 44])
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.0))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(&self.0);
        f.write_str(&s[..6])?;
        f.write_str("...")?;
        f.write_str(&s[36..])?;
        Ok(())
    }
}

impl From<[u8; ADDRESS_LEN]> for Address {
    fn from(slice: [u8; ADDRESS_LEN]) -> Self {
        Address(slice)
    }
}

impl Address {
    pub fn from_slice(slice: &[u8]) -> Self {
        let mut bytes = [0; ADDRESS_LEN];
        bytes.copy_from_slice(slice);
        Self(bytes)
    }

    pub fn from_slice_checked(slice: &[u8]) -> Result<Self, Error> {
        let mut bytes = [0; ADDRESS_LEN];
        if slice.len() != bytes.len() {
            return Err(Error::AddressParseFailed);
        }
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    pub fn hrp(&self) -> String {
        match bech32::decode(&String::from_utf8_lossy(&self.0)) {
            Ok((hrp, _, _)) => hrp,
            Err(_) => String::new(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
    pub fn is_zero(&self) -> bool {
        *self == Address::default()
    }

    pub fn is_default(&self) -> bool {
        *self == Address::default()
    }

    pub fn to_address20(&self) -> Option<H160> {
        match bech32::decode(&String::from_utf8_lossy(&self.0))
            .and_then(|(_, address_32, _)| bech32::convert_bits(&address_32, 5, 8, false))
        {
            Ok(address) => Some(H160::from_slice(&address)),
            Err(_) => None,
        }
    }
}

impl FromStr for Address {
    type Err = bech32::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if !input.len() == ADDRESS_LEN {
            return Err(Self::Err::InvalidLength);
        }
        let _ = bech32::decode(input)?;
        let mut bytes = [0; ADDRESS_LEN];
        bytes.copy_from_slice(input.as_bytes());
        Ok(Address(bytes))
    }
}

impl From<Vec<u8>> for Address {
    fn from(value: Vec<u8>) -> Self {
        let mut bytes = [0; ADDRESS_LEN];
        bytes.copy_from_slice(value.as_slice());
        Address(bytes)
    }
}
