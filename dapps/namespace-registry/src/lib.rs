#![cfg_attr(not(test), no_std)]
extern crate alloc;
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
pub mod genesis;
pub mod service {
    include!(concat!(env!("OUT_DIR"), "/service.rs"));
}
mod util;

use crate::constants::ADMIN;
use crate::genesis::restricted_namespaces;
use once_cell::sync::OnceCell;
use rune_framework::prelude::*;
use service::registry::{RegistryInstance, RegistryService};

#[rune::rune]
pub mod app {
    use super::*;
    use crate::service::{GetNamespaceRequest, NameSpaceRegistered, Namespace, OwnerChanged};
    use crate::util::decode_namespace;
    use primitive_types::{Address, H256};

    #[rune::route(with = "RegistryInstance")]
    pub struct Registry;

    #[rune::storage_map(key_type = "H256", value_type = "Address")]
    pub struct RegisteredNameSpaces;

    impl RegistryService for Registry {
        fn register(call: Call<Namespace>) -> anyhow::Result<NameSpaceRegistered> {
            anyhow::ensure!(call.origin() == ADMIN);
            let ns = call
                .message
                .namespace
                .ok_or_else(|| anyhow::anyhow!("namespace cant be None"))?;
            let owner = call
                .message
                .owner
                .ok_or_else(|| anyhow::anyhow!("owner cant be None"))?;
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

        fn set_owner(call: Call<Namespace>) -> anyhow::Result<OwnerChanged> {
            let Some(new_owner) = call.message.owner else {
                anyhow::bail!("namespace doesnt have owner field set")
            };
            let Some(namespace) = call.message.namespace else {
                anyhow::bail!("namespace doesnt have namespace field set")
            };

            let Some(prev_owner) = RegisteredNameSpaces::get(namespace)? else {
                anyhow::bail!("namespace not found")
            };

            anyhow::ensure!(prev_owner == call.origin());

            RegisteredNameSpaces::put(namespace, new_owner);

            Ok(OwnerChanged {
                namespace: Some(namespace),
                new_owner: Some(new_owner),
                prev_owner: Some(prev_owner),
            })
        }

        fn get_namespace(call: Call<GetNamespaceRequest>) -> anyhow::Result<Namespace> {
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
