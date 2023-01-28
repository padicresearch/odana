use std::fmt::Formatter;
use std::ops::{Add, Mul, Sub};

use crate::MAX_SUPPLY_PRECOMPUTED;

pub const TUC_UNIT: u128 = 1_000_000_000;

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct Chi(u128);

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Debug)]
pub struct Tuc {
    chi: Chi,
}

impl From<Chi> for Tuc {
    fn from(chi: Chi) -> Self {
        Self { chi }
    }
}

impl From<u64> for Tuc {
    fn from(n: u64) -> Self {
        let n = Chi::from(n) * Chi(TUC_UNIT);
        Self::from(n)
    }
}

impl AsRef<Chi> for Tuc {
    fn as_ref(&self) -> &Chi {
        &self.chi
    }
}

impl Chi {
    const MAX: Chi = Chi(MAX_SUPPLY_PRECOMPUTED);
    const MIN: Chi = Chi(0);
}

impl From<u128> for Chi {
    fn from(n: u128) -> Self {
        Self(n)
    }
}

impl From<u64> for Chi {
    fn from(n: u64) -> Self {
        Self(n as u128)
    }
}

impl From<u32> for Chi {
    fn from(n: u32) -> Self {
        Self(n as u128)
    }
}

impl From<u16> for Chi {
    fn from(n: u16) -> Self {
        Self(n as u128)
    }
}

impl From<Chi> for u128 {
    fn from(c: Chi) -> Self {
        c.0
    }
}
impl Eq for Chi {}

impl Sub for Chi {
    type Output = Chi;

    fn sub(self, rhs: Self) -> Self::Output {
        Chi(self.0.saturating_sub(rhs.0))
    }
}

impl Add for Chi {
    type Output = Chi;

    fn add(self, rhs: Self) -> Self::Output {
        let s: Chi = self.0.add(rhs.0).into();
        if s > Self::MAX {
            panic!("max allowed value exceeded")
        }
        s
    }
}

impl Mul for Chi {
    type Output = Chi;

    fn mul(self, rhs: Self) -> Self::Output {
        let s: Chi = self.0.add(rhs.0).into();
        if s > Self::MAX {
            panic!("max allowed value exceeded")
        }
        s
    }
}

impl std::fmt::Display for Chi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}