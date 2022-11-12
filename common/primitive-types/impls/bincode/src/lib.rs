#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub use bincode;

#[macro_export]
macro_rules! impl_uint_bincode {
    ($name: ident, $len: expr) => {
        impl $crate::bincode::Encode for $name {
            fn encode<E: $crate::bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), $crate::bincode::error::EncodeError> {
                let mut bytes = [0u8; $len * 8];
                self.to_little_endian(&mut bytes);
                $crate::bincode::Encode::encode(&bytes, encoder)?;
                Ok(())
            }
        }

        impl $crate::bincode::Decode for $name {
            fn decode<D: $crate::bincode::de::Decoder>(
                decoder: &mut D,
            ) -> core::result::Result<Self, $crate::bincode::error::DecodeError> {
                <[u8; $len * 8] as $crate::bincode::Decode>::decode(decoder)
                    .map(|b| $name::from_little_endian(&b))
            }
        }

        impl<'de> $crate::bincode::BorrowDecode<'de> for $name {
            fn borrow_decode<D: $crate::bincode::de::BorrowDecoder<'de>>(
                decoder: &mut D,
            ) -> core::result::Result<Self, $crate::bincode::error::DecodeError> {
                <[u8; $len * 8] as $crate::bincode::BorrowDecode>::borrow_decode(decoder)
                    .map(|b| $name::from_little_endian(&b))
            }
        }
    };
}
#[macro_export]
macro_rules! impl_fixed_hash_bincode {
    ($name: ident, $len: expr) => {
        impl $crate::bincode::Encode for $name {
            fn encode<E: $crate::bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), $crate::bincode::error::EncodeError> {
                $crate::bincode::Encode::encode(&self.0, encoder)?;
                Ok(())
            }
        }

        impl $crate::bincode::Decode for $name {
            fn decode<D: $crate::bincode::de::Decoder>(
                decoder: &mut D,
            ) -> core::result::Result<Self, $crate::bincode::error::DecodeError> {
                <[u8; $len] as $crate::bincode::Decode>::decode(decoder).map($name)
            }
        }

        impl<'de> $crate::bincode::BorrowDecode<'de> for $name {
            fn borrow_decode<D: $crate::bincode::de::BorrowDecoder<'de>>(
                decoder: &mut D,
            ) -> core::result::Result<Self, $crate::bincode::error::DecodeError> {
                <[u8; $len] as $crate::bincode::BorrowDecode>::borrow_decode(decoder).map($name)
            }
        }
    };
}
