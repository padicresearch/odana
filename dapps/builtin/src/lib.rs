use anyhow::bail;
use primitive_types::address::Address;
use prost::Message;
use prost_reflect::DynamicMessage;
use rune_framework::prelude::RuntimeApplication;
use std::collections::BTreeMap;
use std::sync::Arc;
use traits::{StateDB, WasmVMInstance};
use types::app::AppStateKey;
use types::prelude::{
    get_address_from_package_name, AccountState, ApplicationCall, SignedTransaction,
    TransactionData,
};
use types::util::PackageName;

pub fn build_in_apps() -> Vec<(&'static str, &'static [u8])> {
    vec![(
        "network.odax.nameregistry",
        namespace_registry::WASM_BINARY.unwrap(),
    )]
}

pub fn is_namespace_registered(
    vm: &dyn WasmVMInstance,
    pkn: &PackageName,
    tx: &SignedTransaction,
    state_db: Arc<dyn StateDB>,
) -> anyhow::Result<bool> {
    let app_id = get_address_from_package_name("network.odax.nameregistry", tx.network())?;

    let descriptor = prost_reflect::DescriptorPool::decode(
        namespace_registry::PackageNameRegistry::descriptor(),
    )?;

    let Some(service) = descriptor.get_service_by_name("service.Registry") else {
        bail!("service.Registry not found")
    };

    let Some(get_owner_method) =  service.methods().find(|method| {
        method.name() == "GetOwner"
    })else {
        bail!("GetOwner method not found in NameRegistry service")
    };

    let input = get_owner_method.input();
    let mut message = DynamicMessage::new(input);
    message.transcode_from(&pkn.organisation_id)?;

    let call = ApplicationCall {
        app_id,
        service: rune_framework::prelude::Hashing::twox_64_hash(b"service.Registry"),
        method: rune_framework::prelude::Hashing::twox_64_hash(b"/service.Registry/GetOwner"),
        args: message.encode_to_vec(),
    };

    let raw_output = vm.execute_app_query(state_db, &call)?;
    let owner = Address::decode(raw_output.as_slice())?;
    Ok(!owner.is_zero() && owner != Address::default())
}

pub fn register_namespace(
    vm: &dyn WasmVMInstance,
    states: &mut BTreeMap<Address, AccountState>,
    tx: &SignedTransaction,
    state_db: Arc<dyn StateDB>,
) -> anyhow::Result<()> {
    let TransactionData::Create(arg) = tx.data() else {
        return Ok(())
    };

    let pkn = PackageName::parse(&arg.package_name)?;

    let app_id = get_address_from_package_name("network.odax.nameregistry", tx.network())?;

    let descriptor = prost_reflect::DescriptorPool::decode(
        namespace_registry::PackageNameRegistry::descriptor(),
    )?;

    let Some(service) = descriptor.get_service_by_name("service.Registry") else {
        bail!("service.NameRegistry not found")
    };

    let Some(register_namespace_method) =  service.methods().find(|method| {
        method.name() == "RegisterNameSpace"
    })else {
        bail!("GetOwner method not found in NameRegistry service")
    };

    let input = register_namespace_method.input();
    let mut message = DynamicMessage::new(input);
    message.transcode_from(&namespace_registry::service::Namespace {
        namespace: Some(pkn.organisation_id),
        owner: Some(tx.sender()),
    })?;
    let call = ApplicationCall {
        app_id,
        service: rune_framework::prelude::Hashing::twox_64_hash(b"service.Registry"),
        method: rune_framework::prelude::Hashing::twox_64_hash(b"/service.Registry/Register"),
        args: message.encode_to_vec(),
    };

    let namespace_registry_changes = vm.execute_app_tx(
        state_db.clone(),
        namespace_registry::constants::ADMIN,
        0,
        &call,
    )?;

    for (addr, state) in namespace_registry_changes.account_changes {
        states.insert(addr, state);
    }

    let app_state = states
        .get_mut(&app_id)
        .and_then(|account_state| account_state.app_state.as_mut())
        .ok_or_else(|| anyhow::anyhow!("app state not found"))?;
    app_state.root_hash = namespace_registry_changes.storage.root();

    state_db.set_app_data(
        AppStateKey(app_id, namespace_registry_changes.storage.root()),
        namespace_registry_changes.storage,
    )?;

    Ok(())
}
