use std::ops::{Sub, Add, Mul};

pub const TUCI : u128 = 1_000_000_000;

pub const MAX_MONEY : u128 = 1_000_000_000 * TUCI;

#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct Tuci(u128);

impl From<u128> for Tuci {
    fn from(n: u128) -> Self {
        Self(n)
    }
}

impl Into<u128> for Tuci {
    fn into(self) -> u128 {
        self.0
    }
}

impl Eq for Tuci {}

impl Sub for Tuci {
    type Output = Tuci;

    fn sub(self, rhs: Self) -> Self::Output {
        Tuci(self.0.checked_sub(rhs.0).unwrap_or(0))
    }
}

impl Add for Tuci {
    type Output = Tuci;

    fn add(self, rhs: Self) -> Self::Output {
        Tuci(self.0.checked_add(rhs.0).unwrap_or(MAX_MONEY))
    }
}


impl Mul for Tuci {
    type Output = Tuci;

    fn mul(self, rhs: Self) -> Self::Output {
        Tuci(self.0.checked_mul(rhs.0).unwrap_or(MAX_MONEY))
    }
}

pub trait Saturating {
    fn saturating_add(self, o: Self) -> Self;

    fn saturating_sub(self, o: Self) -> Self;

    fn saturating_mul(self, o: Self) -> Self;

}




/*
pub fn money_range() -> bool {

}*/