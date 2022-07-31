#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnsignedTransaction {
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
    #[prost(string, tag = "2")]
    pub to: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub amount: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub fee: ::prost::alloc::string::String,
}
