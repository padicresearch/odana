#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "std",
    doc = "Odana runtime standard library as compiled when linked with Rust's standard library."
)]
#![cfg_attr(
    not(feature = "std"),
    doc = "Odana runtime standard library as compiled without Rust's standard library."
)]

#[cfg(feature = "std")]
include!("../with_std.rs");

#[cfg(not(feature = "std"))]
include!("../without_std.rs");

/// A target for `core::write!` macro - constructs a string in memory.
#[derive(Default)]
pub struct Writer(vec::Vec<u8>);

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.extend(s.as_bytes());
        Ok(())
    }
}

impl Writer {
    /// Access the content of this `Writer` e.g. for printout
    pub fn inner(&self) -> &vec::Vec<u8> {
        &self.0
    }

    /// Convert into the content of this `Writer`
    pub fn into_inner(self) -> vec::Vec<u8> {
        self.0
    }
}

/// Prelude of common useful imports.
///
/// This should include only things which are in the normal std prelude.
pub mod prelude {
    pub use crate::{
        borrow::ToOwned,
        boxed::Box,
        clone::Clone,
        cmp::{Eq, PartialEq, Reverse},
        iter::IntoIterator,
        vec::Vec,
    };

    // Re-export `vec!` macro here, but not in `std` mode, since
    // std's prelude already brings `vec!` into the scope.
    #[cfg(not(feature = "std"))]
    pub use crate::vec;
}
