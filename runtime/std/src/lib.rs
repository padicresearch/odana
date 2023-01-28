#![no_std]
pub extern crate alloc;

pub use alloc::boxed;
pub use alloc::rc;
pub use alloc::sync;
pub use alloc::vec;
pub use core::any;
pub use core::cell;
pub use core::clone;
pub use core::cmp;
pub use core::convert;
pub use core::default;
pub use core::fmt;
pub use core::hash;
pub use core::iter;
pub use core::marker;
pub use core::mem;
pub use core::num;
pub use core::ops;
pub use core::ptr;
pub use core::result;
pub use core::slice;
pub use core::str;
pub use core::time;

pub mod collections {
    pub use alloc::collections::btree_map;
    pub use alloc::collections::btree_set;
    pub use alloc::collections::vec_deque;
}

pub mod borrow {
    pub use alloc::borrow::*;
    pub use core::borrow::*;
}

pub mod thread {
    pub fn panicking() -> bool {
        false
    }
}

#[derive(Default)]
pub struct Writer(vec::Vec<u8>);

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.extend(s.as_bytes());
        Ok(())
    }
}

impl Writer {
    pub fn inner(&self) -> &vec::Vec<u8> {
        &self.0
    }

    pub fn into_inner(self) -> vec::Vec<u8> {
        self.0
    }
}
pub mod prelude {
    pub use crate::{
        borrow::ToOwned,
        boxed::Box,
        clone::Clone,
        cmp::{Eq, PartialEq, Reverse},
        iter::IntoIterator,
        vec,
        vec::Vec,
    };
}
