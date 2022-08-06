// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use prefix_hex::Error;

#[test]
fn array_decode() {
    assert_eq!(prefix_hex::decode("0xffffff"), Ok([255, 255, 255]));
}

#[test]
fn array_decode_invalid_hex() {
    assert_eq!(
        prefix_hex::decode::<[u8; 3]>("0x00000y"),
        Err(Error::InvalidHexCharacter { c: 'y', index: 5 })
    );
}

#[test]
fn array_decode_invalid_length_too_short() {
    assert_eq!(
        prefix_hex::decode::<[u8; 3]>("0x52fd6"),
        Err(Error::InvalidStringLengthSlice { expected: 6, actual: 5 })
    );
}

#[test]
fn array_decode_invalid_length_too_long() {
    assert_eq!(
        prefix_hex::decode::<[u8; 3]>("0x52fd643"),
        Err(Error::InvalidStringLengthSlice { expected: 6, actual: 7 })
    );
}

#[test]
fn array_decode_no_prefix() {
    assert_eq!(
        prefix_hex::decode::<[u8; 3]>("004200"),
        Err(Error::InvalidPrefix { c0: '0', c1: '0' })
    );
}

#[test]
fn array_decode_wrong_prefix() {
    assert_eq!(
        prefix_hex::decode::<[u8; 3]>("0yffffff"),
        Err(Error::InvalidPrefix { c0: '0', c1: 'y' })
    );
}

#[test]
fn array_encode() {
    assert_eq!(
        prefix_hex::encode([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
        "0x0123456789abcdef"
    );
}

#[test]
fn array_reference_encode() {
    assert_eq!(
        prefix_hex::encode(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
        "0x0123456789abcdef"
    );
}

#[test]
fn boxed_slice_encode() {
    assert_eq!(
        prefix_hex::encode(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef].into_boxed_slice()),
        "0x0123456789abcdef"
    );
}

#[test]
fn boxed_slice_reference_encode() {
    assert_eq!(
        prefix_hex::encode(&vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef].into_boxed_slice()),
        "0x0123456789abcdef"
    );
}

#[test]
fn vec_decode() {
    assert_eq!(prefix_hex::decode::<Vec<u8>>("0x000102").unwrap(), [0x0, 0x1, 0x2]);
}

#[test]
fn vec_decode_empty_string() {
    assert_eq!(prefix_hex::decode("0x"), Ok(vec![]));
}

#[test]
fn vec_decode_odd_length() {
    assert_eq!(prefix_hex::decode::<Vec<u8>>("0xf0f0f"), Err(Error::OddLength));
}

#[test]
fn vec_encode() {
    assert_eq!(
        prefix_hex::encode(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
        "0x0123456789abcdef"
    );
}
