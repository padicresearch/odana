#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Genesis {
    #[prost(bytes = "vec", tag = "1")]
    pub creator: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub names: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Call {
    #[prost(oneof = "call::Arg", tags = "1, 2")]
    pub arg: ::core::option::Option<call::Arg>,
}
/// Nested message and enum types in `Call`.
pub mod call {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Arg {
        #[prost(bytes, tag = "1")]
        SetName(::prost::alloc::vec::Vec<u8>),
        #[prost(bytes, tag = "2")]
        ClearName(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Query {
    #[prost(oneof = "query::Arg", tags = "1")]
    pub arg: ::core::option::Option<query::Arg>,
}
/// Nested message and enum types in `Query`.
pub mod query {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Arg {
        #[prost(bytes, tag = "1")]
        GetName(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Name {
    #[prost(bytes = "vec", tag = "1")]
    pub name: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryResponse {
    #[prost(oneof = "query_response::Arg", tags = "1")]
    pub arg: ::core::option::Option<query_response::Arg>,
}
/// Nested message and enum types in `QueryResponse`.
pub mod query_response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Arg {
        #[prost(message, tag = "1")]
        Name(super::Name),
    }
}
