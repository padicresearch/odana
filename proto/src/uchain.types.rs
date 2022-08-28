#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Empty {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeader {
    #[prost(string, tag="1")]
    pub parent_hash: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub merkle_root: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub state_root: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub mix_nonce: ::prost::alloc::string::String,
    #[prost(string, tag="5")]
    pub coinbase: ::prost::alloc::string::String,
    #[prost(uint32, tag="6")]
    pub difficulty: u32,
    #[prost(uint32, tag="7")]
    pub chain_id: u32,
    #[prost(int32, tag="8")]
    pub level: i32,
    #[prost(uint32, tag="9")]
    pub time: u32,
    #[prost(string, tag="10")]
    pub nonce: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Block {
    #[prost(string, tag="1")]
    pub hash: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub header: ::core::option::Option<BlockHeader>,
    #[prost(message, repeated, tag="3")]
    pub txs: ::prost::alloc::vec::Vec<SignedTransaction>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PaymentTx {
    #[prost(string, tag="1")]
    pub to: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub amount: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AnyType {
    #[prost(string, tag="1")]
    pub type_info: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub value: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApplicationCallTx {
    #[prost(string, tag="1")]
    pub app_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub args: ::core::option::Option<AnyType>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(string, tag="1")]
    pub nonce: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub chain_id: u32,
    #[prost(string, tag="3")]
    pub genesis_hash: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub fee: ::prost::alloc::string::String,
    #[prost(oneof="unsigned_transaction::Data", tags="5, 6, 7")]
    pub data: ::core::option::Option<unsigned_transaction::Data>,
}
/// Nested message and enum types in `UnsignedTransaction`.
pub mod unsigned_transaction {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag="5")]
        Payment(super::PaymentTx),
        #[prost(message, tag="6")]
        Call(super::ApplicationCallTx),
        #[prost(string, tag="7")]
        Raw(::prost::alloc::string::String),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignedTransaction {
    #[prost(string, tag="1")]
    pub nonce: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub chain_id: u32,
    #[prost(string, tag="3")]
    pub genesis_hash: ::prost::alloc::string::String,
    #[prost(string, tag="4")]
    pub fee: ::prost::alloc::string::String,
    #[prost(string, tag="8")]
    pub r: ::prost::alloc::string::String,
    #[prost(string, tag="9")]
    pub s: ::prost::alloc::string::String,
    #[prost(string, tag="10")]
    pub v: ::prost::alloc::string::String,
    #[prost(oneof="signed_transaction::Data", tags="5, 6, 7")]
    pub data: ::core::option::Option<signed_transaction::Data>,
}
/// Nested message and enum types in `SignedTransaction`.
pub mod signed_transaction {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag="5")]
        Payment(super::PaymentTx),
        #[prost(message, tag="6")]
        Call(super::ApplicationCallTx),
        #[prost(string, tag="7")]
        Raw(::prost::alloc::string::String),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionList {
    #[prost(message, repeated, tag="1")]
    pub txs: ::prost::alloc::vec::Vec<SignedTransaction>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountState {
    #[prost(string, tag="1")]
    pub free_balance: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub reserve_balance: ::prost::alloc::string::String,
    #[prost(string, tag="3")]
    pub nonce: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum TransactionStatus {
    Confirmed = 0,
    Pending = 1,
    Queued = 2,
    NotFound = 3,
}
impl TransactionStatus {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TransactionStatus::Confirmed => "Confirmed",
            TransactionStatus::Pending => "Pending",
            TransactionStatus::Queued => "Queued",
            TransactionStatus::NotFound => "NotFound",
        }
    }
}
