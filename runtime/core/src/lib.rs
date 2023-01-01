//!
//!
//! # Example
//! impl RuntimeApplication for ExampleApp {
//!     type Genesis = types::Genesis;
//!     type Call = types::Call;
//!     type Query = types::Query;
//!     type QueryResponse = types::QueryResponse;
//!
//!     fn init(block_level: u32, genesis: Self::Genesis) -> u32 {
//!         ...
//!     }
//!
//!     fn call(context: impl ExecutionContext, call: Self::Call) {
//!         ...
//!     }
//!
//!
//!     fn query(query: Self::Query) -> Self::QueryResponse {
//!         ...
//!     }
//! }
//!
//! export_app!(NickApp);

#![no_std]

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
}
include!(concat!(env!("OUT_DIR"), "/app.rs"));
use odana_std::prelude::*;
use prost::Message;

pub trait ExecutionContext {
    fn block_level(&self) -> u32;
    fn chain_id(&self) -> u32;
    fn miner(&self) -> &[u8];
    fn sender(&self) -> &[u8];
    fn fee(&self) -> u64;
}

impl ExecutionContext for app::Context {
    fn block_level(&self) -> u32 {
        self.block_level
    }

    fn chain_id(&self) -> u32 {
        self.chain_id
    }

    fn miner(&self) -> &[u8] {
        &self.miner
    }

    fn sender(&self) -> &[u8] {
        &self.sender
    }

    fn fee(&self) -> u64 {
        self.fee
    }
}

pub trait RuntimeApplication {
    type Genesis: prost::Message + Default;
    type Call: prost::Message + Default;
    type Query: prost::Message + Default;
    type QueryResponse: prost::Message + Default;

    fn init(block_level: u32, genesis: Self::Genesis) -> u32;
    fn call(context: impl ExecutionContext, call: Self::Call);
    fn query(query: Self::Query) -> Self::QueryResponse;
}

impl<T> app::App for T
    where
        T: RuntimeApplication,
{
    fn init(block_level: u32, genesis: Vec<u8>) -> u32 {
        T::init(
            block_level,
            T::Genesis::decode(genesis.as_slice()).expect("error parse genesis"),
        )
    }

    fn call(c: app::Context, call: Vec<u8>) -> () {
        T::call(
            c,
            T::Call::decode(call.as_slice()).expect("error parsing call"),
        )
    }

    fn query(query: Vec<u8>) -> Vec<u8> {
        T::query(T::Query::decode(query.as_slice()).expect("error parsing query")).encode_to_vec()
    }
}
