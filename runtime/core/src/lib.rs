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
#[doc(hidden)]
include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
use rt_std::prelude::*;
use prost::Message;

pub trait ExecutionContext {
    fn block_level(&self) -> u32;
    fn chain_id(&self) -> u32;
    fn miner(&self) -> &[u8];
    fn sender(&self) -> &[u8];
    fn fee(&self) -> u64;
}

pub trait Runtime {
    type Call: prost::Message + Default;
    type Query: prost::Message + Default;
    type QueryResponse: prost::Message + Default;

    fn genesis();
    fn call(call: Self::Call);
    fn query(query: Self::Query) -> Self::QueryResponse;
}

impl<T> runtime_api::RuntimeApi for T
    where
        T: Runtime,
{
    fn genesis() {
        T::genesis()
    }

    fn call(call: Vec<u8>) -> () {
        T::call(
            T::Call::decode(call.as_slice()).expect("error parsing call"),
        )
    }

    fn query(query: Vec<u8>) -> Vec<u8> {
        T::query(T::Query::decode(query.as_slice()).expect("error parsing query")).encode_to_vec()
    }
}
