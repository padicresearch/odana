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

pub mod io;

extern crate alloc;

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
}
#[doc(hidden)]
include!(concat!(env!("OUT_DIR"), "/app.rs"));
use crate::context::Context;
use primitive_types::{Address, H256};
use prost::Message;
use rune_std::prelude::*;

pub struct GenericBlock {
    pub block_level: u32,
    pub chain_id: u32,
    pub parent_hash: H256,
    pub miner: Address,
}

pub trait RuntimeApplication {
    type Call: prost::Message + Default;
    type Query: prost::Message + Default;
    type QueryResponse: prost::Message + Default;

    /// Initializes the runtime application.
    fn genesis(context: Context) -> anyhow::Result<()>;

    /// Handles a call to the runtime application.
    ///
    /// # Parameters
    ///
    /// - `call`: An instance of the `Call` type representing the call to be handled
    fn call(context: Context, call: Self::Call) -> anyhow::Result<()>;

    /// Handles a query to the runtime application and returns a response.
    ///
    /// # Parameters
    ///
    /// - `query`: An instance of the `Query` type representing the query to be handled
    ///
    /// # Returns
    ///
    /// - An instance of the `QueryResponse` type representing the response to the query
    fn query(query: Self::Query) -> Self::QueryResponse;
}

pub mod context {
    use crate::{app, execution_context, internal};
    use primitive_types::Address;

    pub struct Context;

    impl Context {
        pub fn sender(&self) -> Address {
            Address::from_slice(execution_context::sender().as_slice()).unwrap_or_default()
        }

        pub fn value(&self) -> u64 {
            execution_context::value()
        }

        pub fn block_level(&self) -> u32 {
            execution_context::block_level()
        }
    }
}

pub mod syscall {
    use crate::{internal, GenericBlock};
    use alloc::boxed::Box;
    use core::iter::once;
    use primitive_types::{Address, H256, H512};

    // Returns the block hash at a specific level
    pub fn block_hash(level: u32) -> H256 {
        H256::from_slice(internal::syscall::block_hash(level).as_slice())
    }

    // Returns a block given its hash
    pub fn block(_block_hash: &H256) -> GenericBlock {
        unimplemented!()
    }

    // Returns the address associated with a specific public key
    pub fn address_from_pk(pk: &H256) -> Address {
        Address::from_slice(&internal::syscall::address_from_pk(pk.as_bytes())).unwrap()
    }

    // Generates a new keypair and returns it as a tuple of private and public keys
    pub fn generate_keypair() -> (H256, H256) {
        let (sk, pk) = internal::syscall::generate_keypair();
        (
            H256::from_slice(sk.as_slice()),
            H256::from_slice(pk.as_slice()),
        )
    }

    // Generates a new native address given a seed
    pub fn generate_native_address(seed: &[u8]) -> Address {
        Address::from_slice(&internal::syscall::generate_native_address(seed)).unwrap()
    }

    // Sign a message with a specific private key and returns the signature
    pub fn sign(sk: &H256, msg: &[u8]) -> H512 {
        H512::from_slice(internal::syscall::sign(sk.as_bytes(), msg).as_slice())
    }

    // Transfers an amount of funds to a specific address
    pub fn transfer(to: &Address, amount: u64) -> bool {
        internal::syscall::transfer(to.as_bytes(), amount)
    }

    // Reserve an amount of funds
    pub fn reserve(amount: u64) -> bool {
        internal::syscall::reserve(amount)
    }
}

impl<T> app::App for T
    where
        T: RuntimeApplication,
{
    fn genesis() {
        T::genesis(Context).unwrap()
    }

    fn call(call: Vec<u8>) -> () {
        T::call(
            Context,
            T::Call::decode(call.as_slice()).expect("error parsing call"),
        )
            .unwrap()
    }

    fn query(query: Vec<u8>) -> Vec<u8> {
        T::query(T::Query::decode(query.as_slice()).expect("error parsing query")).encode_to_vec()
    }
}
