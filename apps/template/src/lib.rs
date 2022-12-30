#![no_std]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
pub mod types;

use odana_core::*;
use odana_std::prelude::*;

struct NickApp;

impl RuntimeApplication for NickApp {
    type Genesis = types::Genesis;
    type Call = types::Call;
    type Query = types::Query;
    type QueryResponse = types::QueryResponse;

    fn init(block_level: u32, genesis: Self::Genesis) -> u32 {
        todo!()
    }

    fn call(context: ExecutionContext, call: Self::Call) {
        todo!()
    }

    fn query(query: Self::Query) -> Self::QueryResponse {
        todo!()
    }
}

export_app!(NickApp);