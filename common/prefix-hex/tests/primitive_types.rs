// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![cfg(all(feature = "primitive-types"))]

macro_rules! test_impl {
    ($name:ident, $type:ty) => {
        ::paste::item! {
            #[test]
            fn [< $name _decode_zero >]() {
                assert_eq!(
                    prefix_hex::decode("0x0"),
                    Ok(<$type>::zero())
                );
            }

            #[test]
            fn [< $name _decode_err_no_body >]() {
                assert_eq!(
                    prefix_hex::decode::<$type>("0x"),
                    Err(prefix_hex::Error::InvalidStringLength)
                );
            }

            #[test]
            fn [< $name _decode_invalid_character >]() {
                assert_eq!(
                    prefix_hex::decode::<$type>("0x271y"),
                    Err(prefix_hex::Error::InvalidHexCharacter{index: 3, c: 'y'})
                );
            }

            #[test]
            fn [< $name _decode_truncated_even >]() {
                assert_eq!(prefix_hex::decode("0x2710"), Ok(<$type>::from(10000)));
            }

            #[test]
            fn [< $name _decode_truncated_odd >]() {
                assert_eq!(prefix_hex::decode("0x102"), Ok(<$type>::from(258)));
            }

            #[test]
            fn [< $name _encode >]() {
                assert_eq!(prefix_hex::encode(<$type>::from(10000)), "0x2710".to_string())
            }
        }
    };
}

test_impl!(u128, primitive_types::U128);
test_impl!(u256, primitive_types::U256);
test_impl!(u512, primitive_types::U512);
