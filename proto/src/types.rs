#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Empty {}

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeader {
    #[prost(string, tag = "1")]
    pub parent_hash: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub merkle_root: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub state_root: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub mix_nonce: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub coinbase: ::prost::alloc::string::String,
    #[prost(uint32, tag = "6")]
    pub difficulty: u32,
    #[prost(uint32, tag = "7")]
    pub chain_id: u32,
    #[prost(int32, tag = "8")]
    pub level: i32,
    #[prost(uint32, tag = "9")]
    pub time: u32,
    #[prost(string, tag = "10")]
    pub nonce: ::prost::alloc::string::String,
}

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
