#![cfg_attr(not(test), no_std)]
extern crate alloc;
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub const PACKAGE_NAME: &'static str = "network.odax.namespace.registry";

pub mod constants;
mod genesis;
mod store;

use crate::constants::ADMIN;
use crate::genesis::restricted_namespaces;
use crate::store::RegisteredNameSpaces;
use crate::types::call::Data;
use crate::types::GetOwnerResponse;
use rune_framework::context::Context;
use rune_framework::io::StorageMap;
use rune_framework::*;

const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/file_descriptor_set.bin"));

#[allow(unused_imports)]
#[allow(dead_code)]
pub mod types {
    include!(concat!(
        env!("OUT_DIR"),
        "/network.odax.namespace.registry.types.rs"
    ));
}

pub struct PackageNameRegistry;

impl RuntimeApplication for PackageNameRegistry {
    type Call = types::Call;
    type Query = types::Query;
    type QueryResponse = types::QueryResponse;

    fn genesis(_: Context) -> anyhow::Result<()> {
        for ns in restricted_namespaces() {
            RegisteredNameSpaces::put(ns, ADMIN)
        }
        Ok(())
    }

    fn call(context: Context, call: Self::Call) -> anyhow::Result<()> {
        let Some(data) = call.data else {
            return Ok(());
        };
        match data {
            Data::Register(info) => {
                anyhow::ensure!(context.sender() == ADMIN);
                let ns = info
                    .namespace
                    .ok_or(anyhow::anyhow!("namespace cant be None"))?;
                let owner = info.owner.ok_or(anyhow::anyhow!("owner cant be None"))?;
                if RegisteredNameSpaces::contains(ns) {
                    panic!("namespace already registered")
                }
                RegisteredNameSpaces::put(ns, owner)
            }
        }
        Ok(())
    }

    fn query(query: Self::Query) -> Self::QueryResponse {
        let Some(data) = query.data else {
            return Self::QueryResponse::default();
        };
        match data {
            types::query::Data::GetOwner(query) => {
                let Some(ns) = query.namespace else {
                    return Self::QueryResponse::default();
                };
                let address = RegisteredNameSpaces::get(ns)
                    .unwrap_or_default()
                    .map(|address| {
                        types::query_response::Data::Owner(GetOwnerResponse {
                            address: Some(address),
                        })
                    });
                Self::QueryResponse { data: address }
            }
        }
    }

    fn descriptor() -> &'static [u8] {
        DESCRIPTOR
    }
}

export_app!(PackageNameRegistry);
