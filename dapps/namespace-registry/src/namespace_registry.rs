#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "namespace_registry.Namespace")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Namespace {
    #[prost(message, optional, tag = "1")]
    pub namespace: ::core::option::Option<::primitive_types::H256>,
    #[prost(message, optional, tag = "2")]
    pub owner: ::core::option::Option<::primitive_types::Address>,
}
#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "namespace_registry.NameSpaceRegistered")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NameSpaceRegistered {
    #[prost(message, optional, tag = "1")]
    pub namespace: ::core::option::Option<Namespace>,
}
#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "namespace_registry.GetNamespaceInfoRequest")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNamespaceInfoRequest {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
}
/// Generated server implementations.
pub mod name_registry {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use rune_framework::prelude::*;
    use rune_std::marker::PhantomData;
    /// Generated trait containing gRPC methods that should be implemented for use with NameRegistryServer.
    pub trait NameRegistryService: Send + Sync + 'static {
        fn register_namespace(
            call: Call<super::Namespace>,
        ) -> ::anyhow::Result<super::NameSpaceRegistered>;
        fn get_owner(
            call: Call<::primitive_types::H256>,
        ) -> ::anyhow::Result<::primitive_types::Address>;
        fn get_namespace_info(
            call: Call<super::GetNamespaceInfoRequest>,
        ) -> ::anyhow::Result<super::Namespace>;
    }
    #[derive(Debug)]
    pub struct NameRegistryInstance<T: NameRegistryService> {
        inner: PhantomData<T>,
    }
    impl<T: NameRegistryService> NameRegistryInstance<T> {
        pub fn new() -> Self {
            Self {
                inner: PhantomData::default(),
            }
        }
    }
    impl<T> Service for NameRegistryInstance<T>
    where
        T: NameRegistryService,
    {
        fn call(&self, method: u64, payload: &[u8]) -> CallResponse {
            if Hashing::twox_64_hash(b"/namespace_registry.NameRegistry/RegisterNamespace")
                == method
            {
                return T::register_namespace(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            if Hashing::twox_64_hash(b"/namespace_registry.NameRegistry/GetOwner") == method {
                return T::get_owner(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            if Hashing::twox_64_hash(b"/namespace_registry.NameRegistry/GetNamespaceInfo") == method
            {
                return T::get_namespace_info(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            return CallResponse::default();
        }
    }
    impl<T: NameRegistryService> rune_framework::NamedService for NameRegistryInstance<T> {
        const NAME: &'static str = "namespace_registry.NameRegistry";
    }
}
