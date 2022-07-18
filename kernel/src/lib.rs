#![feature(map_first_last)]

mod sync;
pub mod messages;
mod miner;

pub(crate) struct KernelContext {}

pub use sync::*;