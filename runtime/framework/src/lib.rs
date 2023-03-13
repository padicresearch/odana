#![no_std]
pub mod io;
extern crate alloc;

#[doc(hidden)]
mod internal {
    include!(concat!(env!("OUT_DIR"), "/system.rs"));
}

#[doc(hidden)]
mod runtime {
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}

use crate::context::Context;
use crate::io::Hashing;
use alloc::collections::BTreeMap;
use primitive_types::Address;
use prost::DecodeError;
use rune_std::prelude::*;

pub struct Call<T: prost::Message + Default> {
    pub message: T,
}

impl<T: prost::Message + Default> Call<T> {
    pub fn new<B: prost::bytes::Buf>(raw_message: B) -> Result<Self, DecodeError> {
        T::decode(raw_message).map(|message| Call { message })
    }

    pub fn origin(&self) -> Address {
        Address::from_slice(runtime::execution_context::sender().as_slice())
    }

    pub fn value(&self) -> u64 {
        runtime::execution_context::value()
    }

    pub fn block_level(&self) -> u32 {
        runtime::execution_context::block_level()
    }
}

pub struct CallResponse {
    type_descriptor: &'static str,
    data: Vec<u8>,
}
impl<T> From<T> for CallResponse
where
    T: prost_extra::MessageExt,
{
    fn from(value: T) -> Self {
        CallResponse {
            type_descriptor: T::full_name(),
            data: value.encode_to_vec(),
        }
    }
}

impl Default for CallResponse {
    fn default() -> Self {
        Self {
            type_descriptor: "",
            data: vec![],
        }
    }
}

pub trait Service {
    fn call(&self, method: u64, payload: &[u8]) -> CallResponse;
}

pub trait NamedService {
    const NAME: &'static str;
}

pub struct Router {
    services: BTreeMap<u64, Box<dyn Service + Send + Sync>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            services: Default::default(),
        }
    }

    pub fn register_service<T: Service + NamedService + Send + Sync + 'static>(
        &mut self,
        service: T,
    ) {
        self.services
            .insert(Hashing::twox_64_hash(T::NAME.as_ref()), Box::new(service));
    }

    pub fn handle(&self, context: Context, payload: &[u8]) -> CallResponse {
        let service = self.services.get(&context.service).unwrap();
        service.call(context.method, payload)
    }
}

pub trait RuntimeApplication {
    fn call(context: Context, arg: &[u8]) -> anyhow::Result<CallResponse>;
    fn descriptor() -> &'static [u8];
}

pub trait Genesis {
    /// Initializes the runtime application.
    fn genesis() -> anyhow::Result<()> {
        Ok(())
    }
}

mod context {
    pub struct Context {
        pub(crate) service: u64,
        pub(crate) method: u64,
    }
}

pub mod syscall {
    use crate::internal;

    use primitive_types::address::Address;
    use primitive_types::{H256, H512};

    // Returns the block hash at a specific level
    pub fn block_hash(level: u32) -> H256 {
        H256::from_slice(internal::syscall::block_hash(level).as_slice())
    }

    // Returns the address associated with a specific public key
    pub fn address_from_pk(pk: &H256) -> Address {
        Address::from_slice(&internal::syscall::address_from_pk(pk.as_bytes()))
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
        Address::from_slice(&internal::syscall::generate_native_address(seed))
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

    pub fn unreserve(amount: u64) -> bool {
        internal::syscall::unreserve(amount)
    }

    pub fn get_free_balance(address: &Address) -> u64 {
        internal::syscall::get_free_balance(address.as_bytes())
    }

    pub fn get_reserve_balance(address: &Address) -> u64 {
        internal::syscall::get_reserve_balance(address.as_bytes())
    }

    pub fn get_nonce(address: &Address) -> u64 {
        internal::syscall::get_nonce(address.as_bytes())
    }
}

impl<T> runtime::runtime_app::RuntimeApp for T
where
    T: RuntimeApplication + Genesis,
{
    fn genesis() {
        T::genesis().unwrap()
    }

    fn call(service: u64, method: u64, call: Vec<u8>) {
        let response = T::call(Context { service, method }, call.as_slice()).unwrap();
        io::emit_raw_event(response.type_descriptor, response.data.as_slice())
    }

    fn query(service: u64, method: u64, query: Vec<u8>) -> Vec<u8> {
        let response = T::call(Context { service, method }, query.as_slice()).unwrap();
        response.data
    }

    fn descriptor() -> Vec<u8> {
        T::descriptor().to_vec()
    }
}

pub mod prelude {
    pub use crate::context::*;
    pub use crate::io::*;
    pub use crate::runtime::*;
    pub use crate::{
        Call, CallResponse, Genesis, NamedService, Router, RuntimeApplication, Service,
    };
}
