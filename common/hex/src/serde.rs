use crate::{FromHex, ToHex};
use alloc::string::String;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer, T: ToHex>(t: &T, serializer: S) -> Result<S::Ok, S::Error> {
    let encoded_hex_string = t.encode_hex();
    encoded_hex_string.serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>, T: FromHex>(deserializer: D) -> Result<T, D::Error> {
    let encoded_hex_string = String::deserialize(deserializer)?;
    T::from_hex(&encoded_hex_string).map_err(serde::de::Error::custom)
}
