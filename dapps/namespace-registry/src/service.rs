#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "service.Namespace")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Namespace {
    #[prost(message, optional, tag = "1")]
    pub namespace: ::core::option::Option<::primitive_types::H256>,
    #[prost(message, optional, tag = "2")]
    pub owner: ::core::option::Option<::primitive_types::Address>,
}
#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "service.NameSpaceRegistered")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NameSpaceRegistered {
    #[prost(message, optional, tag = "1")]
    pub namespace: ::core::option::Option<Namespace>,
}
#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "service.OwnerChanged")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OwnerChanged {
    #[prost(message, optional, tag = "1")]
    pub namespace: ::core::option::Option<::primitive_types::H256>,
    #[prost(message, optional, tag = "2")]
    pub new_owner: ::core::option::Option<::primitive_types::Address>,
    #[prost(message, optional, tag = "3")]
    pub prev_owner: ::core::option::Option<::primitive_types::Address>,
}
#[derive(::prost_extra::MessageExt)]
#[prost_extra(message_name = "service.GetNamespaceRequest")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNamespaceRequest {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
}
/// Generated server implementations.
pub mod registry {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use rune_framework::prelude::*;
    use rune_std::marker::PhantomData;
    /// Generated trait containing gRPC methods that should be implemented for use with RegistryServer.
    pub trait RegistryService: Send + Sync + 'static {
        fn register(call: Call<super::Namespace>) -> ::anyhow::Result<super::NameSpaceRegistered>;
        fn get_owner(
            call: Call<::primitive_types::H256>,
        ) -> ::anyhow::Result<::primitive_types::Address>;
        fn set_owner(call: Call<super::Namespace>) -> ::anyhow::Result<super::OwnerChanged>;
        fn get_namespace(
            call: Call<super::GetNamespaceRequest>,
        ) -> ::anyhow::Result<super::Namespace>;
    }
    #[derive(Debug)]
    pub struct RegistryInstance<T: RegistryService> {
        inner: PhantomData<T>,
    }
    impl<T: RegistryService> RegistryInstance<T> {
        pub fn new() -> Self {
            Self {
                inner: PhantomData::default(),
            }
        }
    }
    impl<T> Service for RegistryInstance<T>
    where
        T: RegistryService,
    {
        fn call(&self, method: u64, payload: &[u8]) -> CallResponse {
            if Hashing::twox_64_hash(b"/service.Registry/Register") == method {
                return T::register(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            if Hashing::twox_64_hash(b"/service.Registry/GetOwner") == method {
                return T::get_owner(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            if Hashing::twox_64_hash(b"/service.Registry/SetOwner") == method {
                return T::set_owner(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            if Hashing::twox_64_hash(b"/service.Registry/GetNamespace") == method {
                return T::get_namespace(Call::new(payload).unwrap())
                    .map(|response| CallResponse::from(response))
                    .unwrap_or_default();
            }
            return CallResponse::default();
        }
    }
    impl<T: RegistryService> rune_framework::NamedService for RegistryInstance<T> {
        const NAME: &'static str = "service.Registry";
    }
}
