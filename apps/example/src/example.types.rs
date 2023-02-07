#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.ReservationInfo"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReservationInfo {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub fee: u64,
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.SetName"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetName {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.ClearName"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClearName {}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.GetName"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetName {
    #[prost(bytes = "vec", tag = "1")]
    pub owner: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.Call"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Call {
    #[prost(oneof = "call::Data", tags = "1, 2")]
    pub data: ::core::option::Option<call::Data>,
}
/// Nested message and enum types in `Call`.
pub mod call {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        SetName(super::SetName),
        #[prost(message, tag = "2")]
        ClearName(super::ClearName),
    }
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.Query"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Query {
    #[prost(oneof = "query::Data", tags = "1")]
    pub data: ::core::option::Option<query::Data>,
}
/// Nested message and enum types in `Query`.
pub mod query {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        GetName(super::GetName),
    }
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.QueryResponse"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryResponse {
    #[prost(oneof = "query_response::Data", tags = "1")]
    pub data: ::core::option::Option<query_response::Data>,
}
/// Nested message and enum types in `QueryResponse`.
pub mod query_response {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        Info(super::ReservationInfo),
    }
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.NameChangedEvent"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NameChangedEvent {
    #[prost(bytes = "vec", tag = "1")]
    pub who: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.NameSetEvent"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NameSetEvent {
    #[prost(bytes = "vec", tag = "1")]
    pub who: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::prost_reflect::ReflectMessage)]
#[prost_reflect(
    descriptor_pool = "crate::DESCRIPTOR_POOL",
    message_name = "example.types.Event"
)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Event {
    #[prost(oneof = "event::Data", tags = "1, 2")]
    pub data: ::core::option::Option<event::Data>,
}
/// Nested message and enum types in `Event`.
pub mod event {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        NameSet(super::NameSetEvent),
        #[prost(message, tag = "2")]
        NameChange(super::NameChangedEvent),
    }
}
