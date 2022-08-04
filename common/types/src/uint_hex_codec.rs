use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::Error;
use primitive_types::{U128};

pub fn serialize<S: Serializer, T: num_traits::ToPrimitive>(
    t: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let x = U128::from(t.to_u128().unwrap());
    x.serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>, T: FromStr>(
    deserializer: D,
) -> Result<T, D::Error> {
    let x = U128::deserialize(deserializer)?;
    let s = x.to_string();
    s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
}