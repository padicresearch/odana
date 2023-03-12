#![cfg_attr(not(test), no_std)]
extern crate alloc;
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
mod genesis;
mod namespace_registry;
mod util;

use crate::constants::ADMIN;
use crate::genesis::restricted_namespaces;
use namespace_registry::name_registry::{NameRegistryInstance, NameRegistryService};
use once_cell::sync::OnceCell;
use rune_framework::prelude::*;

#[rune::rune]
pub mod app {
    use super::*;
    use crate::namespace_registry::{GetNamespaceInfoRequest, NameSpaceRegistered, Namespace};
    use crate::util::decode_namespace;
    use primitive_types::{Address, H256};

    #[rune::route(with = "NameRegistryInstance")]
    pub struct NameRegistry;

    #[rune::storage_map(key_type = "H256", value_type = "Address")]
    pub struct RegisteredNameSpaces;

    impl NameRegistryService for NameRegistry {
        fn register_namespace(call: Call<Namespace>) -> anyhow::Result<NameSpaceRegistered> {
            anyhow::ensure!(call.origin() == ADMIN);
            let ns = call
                .message
                .namespace
                .ok_or(anyhow::anyhow!("namespace cant be None"))?;
            let owner = call
                .message
                .owner
                .ok_or(anyhow::anyhow!("owner cant be None"))?;
            if RegisteredNameSpaces::contains(ns) {
                panic!("namespace already registered")
            }
            RegisteredNameSpaces::put(ns, owner);
            Ok(NameSpaceRegistered {
                namespace: Some(call.message),
            })
        }

        fn get_owner(call: Call<H256>) -> anyhow::Result<Address> {
            let namespace = call.message;
            let address = RegisteredNameSpaces::get(namespace)?.unwrap_or_default();
            Ok(address)
        }

        fn get_namespace_info(call: Call<GetNamespaceInfoRequest>) -> anyhow::Result<Namespace> {
            let namespace = decode_namespace(&call.message.namespace)?;
            Ok(Namespace {
                namespace: Some(namespace),
                owner: RegisteredNameSpaces::get(namespace).unwrap_or_default(),
            })
        }
    }

    #[rune::app]
    pub struct PackageNameRegistry;

    impl Genesis for PackageNameRegistry {
        fn genesis() -> anyhow::Result<()> {
            for ns in restricted_namespaces() {
                RegisteredNameSpaces::put(ns, ADMIN)
            }
            Ok(())
        }
    }
}
