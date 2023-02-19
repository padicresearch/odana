#![cfg_attr(not(test), no_std)]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use crate::types::call::Data;
use crate::types::{query, query_response, GetName, QueryResponse, ReservationInfo};
use once_cell::sync::Lazy;
use primitive_types::Address;
use prost_reflect::DescriptorPool;
use rune_framework::context::Context;
use rune_framework::io::{Blake2bHasher, StorageMap, StorageValue};
use rune_framework::*;

const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

static DESCRIPTOR_POOL: Lazy<DescriptorPool> =
    Lazy::new(|| DescriptorPool::decode(DESCRIPTOR).unwrap());

#[allow(unused_imports)]
#[allow(dead_code)]
#[path = "./example.types.rs"]
pub mod types;

struct Nick;

struct AddressReservationInfo;

struct ReservationFee;

impl StorageMap<Blake2bHasher, Address, ReservationInfo> for AddressReservationInfo {
    fn storage_prefix() -> &'static [u8] {
        b"AddressReservationInfo"
    }
}

impl StorageValue<Blake2bHasher, u64> for ReservationFee {
    fn storage_prefix() -> &'static [u8] {
        b"ReservationFee"
    }
}

impl RuntimeApplication for Nick {
    type Call = types::Call;
    type Query = types::Query;
    type QueryResponse = types::QueryResponse;

    fn genesis(_: Context) -> anyhow::Result<()> {
        ReservationFee::set(10 * 100_000);
        Ok(())
    }

    fn call(context: Context, call: Self::Call) -> anyhow::Result<()> {
        let sender = context.sender();
        let Some(data) = call.data else {
            return Ok(());
        };
        match data {
            Data::SetName(param) => {
                let fee = if let Some(info) = AddressReservationInfo::get(sender)? {
                    info.fee
                } else {
                    ReservationFee::get()?
                };
                anyhow::ensure!(rune_framework::syscall::reserve(fee));
                AddressReservationInfo::put(
                    sender,
                    ReservationInfo {
                        name: param.name,
                        fee,
                    },
                );
                anyhow::ensure!(AddressReservationInfo::contains(sender));
            }
            Data::ClearName(_) => {
                AddressReservationInfo::remove(sender)?;
            }
        }
        Ok(())
    }

    fn query(query: Self::Query) -> Self::QueryResponse {
        let response = match query.data {
            Some(query::Data::GetName(GetName { owner })) => {
                if let Ok(Some(data)) = Address::from_slice(&owner)
                    .map_err(|_| ())
                    .and_then(|owner| AddressReservationInfo::get(owner).map_err(|_| ()))
                {
                    QueryResponse {
                        data: Some(query_response::Data::Info(data)),
                    }
                } else {
                    QueryResponse::default()
                }
            }
            _ => QueryResponse::default(),
        };
        response
    }

    fn descriptor() -> &'static [u8] {
        DESCRIPTOR
    }
}

export_app!(Nick);
