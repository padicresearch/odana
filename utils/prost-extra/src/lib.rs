pub trait MessageExt: prost::Message {
    fn full_name(&self) -> &'static str;
}

#[cfg(feature = "derive")]
pub use prost_extra_derive::MessageExt;
