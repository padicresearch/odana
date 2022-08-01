#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(string, tag = "1")]
    pub nonce: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub to: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub amount: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub fee: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub data: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transaction {
    #[prost(string, tag = "1")]
    pub nonce: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub from: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub to: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub amount: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub fee: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub data: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub r: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub s: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub v: ::prost::alloc::string::String,
}
