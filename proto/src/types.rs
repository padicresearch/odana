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
pub struct Block {
    #[prost(string, tag = "1")]
    pub hash: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub header: ::core::option::Option<BlockHeader>,
    #[prost(message, repeated, tag = "3")]
    pub txs: ::prost::alloc::vec::Vec<Transaction>,
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

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Transaction {
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
    #[prost(string, tag = "6")]
    pub r: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub s: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub v: ::prost::alloc::string::String,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionList {
    #[prost(message, repeated, tag = "1")]
    pub txs: ::prost::alloc::vec::Vec<Transaction>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RawBlockHeaderPacket {
    #[prost(bytes = "vec", tag = "1")]
    pub parent_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub merkle_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub state_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub mix_nonce: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub coinbase: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub difficulty: u32,
    #[prost(uint32, tag = "7")]
    pub chain_id: u32,
    #[prost(int32, tag = "8")]
    pub level: i32,
    #[prost(uint32, tag = "9")]
    pub time: u32,
    #[prost(bytes = "vec", tag = "10")]
    pub nonce: ::prost::alloc::vec::Vec<u8>,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountState {
    #[prost(string, tag = "1")]
    pub free_balance: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub reserve_balance: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub nonce: ::prost::alloc::string::String,
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TransactionStatus {
    Confirmed = 0,
    Pending = 1,
    Queued = 2,
    NotFound = 3,
}
