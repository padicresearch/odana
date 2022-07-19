#![feature(map_first_last)]

pub mod messages;
mod miner;

pub(crate) struct KernelContext {}

pub use node::sync::*;