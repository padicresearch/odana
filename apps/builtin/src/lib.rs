use primitive_types::address::Address;
use prost::Message;
use std::collections::BTreeMap;
use std::sync::Arc;
use traits::{StateDB, WasmVMInstance};
use types::app::AppStateKey;
use types::prelude::{
    get_address_from_package_name, AccountState, ApplicationCallTx, SignedTransaction,
    TransactionData,
};
use types::util::PackageName;

pub fn build_in_apps() -> Vec<(&'static str, &'static [u8])> {
    vec![(
        namespace_registry::PACKAGE_NAME,
        namespace_registry::WASM_BINARY.unwrap(),
    )]
}

pub fn is_namespace_registered(
    vm: &dyn WasmVMInstance,
    pkn: &PackageName,
    tx: &SignedTransaction,
    state_db: Arc<dyn StateDB>,
) -> anyhow::Result<bool> {
    let query = namespace_registry::types::Query {
        data: Some(namespace_registry::types::query::Data::GetOwner(
            namespace_registry::types::GetOwner {
                namespace: Some(pkn.organisation_id),
            },
        )),
    }
    .encode_to_vec();

    let namespace_app_id =
        get_address_from_package_name(namespace_registry::PACKAGE_NAME, tx.network())?;

    let (_, raw_response) = vm.execute_app_query(state_db.clone(), namespace_app_id, &query)?;

    let query_response = namespace_registry::types::QueryResponse::decode(raw_response.as_slice())?;

    if let Some(namespace_registry::types::query_response::Data::Owner(owner)) = query_response.data
    {
        let owner = owner.address.unwrap_or_default();
        if !owner.is_default() {
            return Ok(owner == tx.from());
        }
        return Ok(true);
    }
    Ok(false)
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

    if is_namespace_registered(vm, &pkn, tx, state_db.clone())? {
        return Ok(());
    }

    let namespace_app_id =
        get_address_from_package_name(namespace_registry::PACKAGE_NAME, tx.network())?;

    // Insert a new namespace owner
    let call = namespace_registry::types::Call {
        data: Some(namespace_registry::types::call::Data::Register(
            namespace_registry::types::Register {
                namespace: Some(pkn.organisation_id),
                owner: Some(tx.from()),
            },
        )),
    };

    let namespace_registry_changes = vm.execute_app_tx(
        state_db.clone(),
        namespace_registry::constants::ADMIN,
        0,
        &ApplicationCallTx {
            app_id: namespace_app_id,
            args: call.encode_to_vec(),
        },
    )?;

    for (addr, state) in namespace_registry_changes.account_changes {
        states.insert(addr, state);
    }

    let app_state = states
        .get_mut(&namespace_app_id)
        .and_then(|account_state| account_state.app_state.as_mut())
        .ok_or_else(|| anyhow::anyhow!("app state not found"))?;
    app_state.root_hash = namespace_registry_changes.storage.root();

    state_db.set_app_data(
        AppStateKey(namespace_app_id, namespace_registry_changes.storage.root()),
        namespace_registry_changes.storage,
    )?;
    Ok(())
}
