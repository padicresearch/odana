use std::fmt::Formatter;
use std::ops::{Add, Mul, Sub};

use traits::Saturating;


use crate::MAX_SUPPLY_PRECOMPUTED;

pub const TUC_UNIT: u128 = 1_000_000_000;

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct Chi(u128);

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
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
// impl From<f64> for Tuc {
//     fn from(f: f64) -> Self {
//         f
//         todo!()
//     }
// }

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

impl Into<u128> for Chi {
    fn into(self) -> u128 {
        self.0
    }
}

impl Eq for Chi {}

impl Sub for Chi {
    type Output = Chi;

    fn sub(self, rhs: Self) -> Self::Output {
        Chi(self.0.checked_sub(rhs.0).unwrap_or(0))
    }
}

impl Add for Chi {
    type Output = Chi;

    fn add(self, rhs: Self) -> Self::Output {
        let s: Chi = self.0.add(rhs.0).into();
        if s > Self::MAX {
            panic!("max allowed value exceeded")
        }
        s.into()
    }
}

impl Mul for Chi {
    type Output = Chi;

    fn mul(self, rhs: Self) -> Self::Output {
        let s: Chi = self.0.add(rhs.0).into();
        if s > Self::MAX {
            panic!("max allowed value exceeded")
        }
        s.into()
    }
}

impl Saturating for Chi {
    fn saturating_add(self, rhs: Self) -> Self {
        let s: Chi = self.0.checked_add(rhs.0).unwrap_or(Self::MAX.0).into();
        if s > Self::MAX {
            return Self::MAX;
        }
        s.into()
    }

    fn saturating_sub(self, rhs: Self) -> Self {
        let s = self.0.checked_sub(rhs.0).unwrap_or(Self::MIN.0);
        s.into()
    }

    fn saturating_mul(self, rhs: Self) -> Self {
        let s: Chi = self.0.checked_mul(rhs.0).unwrap_or(Self::MAX.0).into();
        if s > Self::MAX {
            return Self::MAX;
        }
        s.into()
    }
}

impl std::fmt::Display for Chi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod test {
    use traits::Saturating;

    use crate::coin::Chi;

    #[test]
    fn tuc_test() {
        println!("{}", Chi(10).saturating_add(Chi(1)));
        println!("{}", Chi(0) + 80_u32.into());
    }
}
