use crate::env::{ExecutionEnvironment, QueryEnvironment};
use anyhow::anyhow;
use internal::Runtime;
use parking_lot::RwLock;
use primitive_types::address::Address;
use smt::SparseMerkleTree;
use std::collections::BTreeMap;
use std::sync::Arc;
use traits::{ChainHeadReader, StateDB, WasmVMInstance};
use wasmtime::component::{Component, Linker};
use wasmtime::{AsContextMut, Config, Engine, Store};
mod env;

use types::account::get_address_from_seed;
use types::prelude::{ApplicationCall, CreateApplication};
use types::{Addressing, Changelist};

#[allow(clippy::all)]
#[allow(dead_code)]
mod internal {
    include!(concat!(env!("OUT_DIR"), "/system.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}
type AppStateStore = BTreeMap<Address, (Runtime, Store<ExecutionEnvironment>)>;
pub struct WasmVM {
    engine: Arc<Engine>,
    blockchain: Arc<dyn ChainHeadReader>,
    apps: Arc<RwLock<AppStateStore>>,
}

impl WasmVM {
    pub fn new(blockchain: Arc<dyn ChainHeadReader>) -> anyhow::Result<Self> {
        Engine::new(Config::new().consume_fuel(false).wasm_component_model(true)).map(|engine| {
            Self {
                engine: Arc::new(engine),
                blockchain,
                apps: Arc::new(Default::default()),
            }
        })
    }

    pub fn create_application(
        &self,
        state_db: Arc<dyn StateDB>,
        origin: Address,
        app_id: Address,
        value: u64,
        binary: &[u8],
    ) -> anyhow::Result<(Vec<u8>, Changelist)> {
        let engine = &self.engine;
        let storage = SparseMerkleTree::new();
        let mut store = Store::new(
            engine,
            ExecutionEnvironment::new(
                origin,
                app_id,
                value,
                storage,
                state_db,
                self.blockchain.clone(),
            )?,
        );

        let mut linker = Linker::<ExecutionEnvironment>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::Io::add_to_linker(&mut linker, |env| env)?;
        let component = Component::from_binary(engine, binary)?;
        let instance = linker.instantiate(&mut store, &component)?;
        let app = Runtime::new(&mut store, &instance)?;
        let app = app.runtime_app();
        app.genesis(&mut store)?;
        let descriptor = app.descriptor(&mut store)?;
        let env = store.data();
        Ok((descriptor, env.into()))
    }

    pub fn install_builtin(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address, //TODO; use codehash instead of app id
        binary: &[u8],
        genesis: bool,
    ) -> anyhow::Result<Option<Changelist>> {
        {
            let apps = self.apps.read();
            if apps.contains_key(&app_id) {
                return Ok(None);
            }
        }
        let engine = &self.engine;
        let storage = if genesis {
            state_db.get_app_data(app_id).unwrap_or_default()
        } else {
            state_db.get_app_data(app_id)?
        };

        let mut store = Store::new(
            engine,
            ExecutionEnvironment::new(
                Default::default(),
                app_id,
                0,
                storage,
                state_db.clone(),
                self.blockchain.clone(),
            )?,
        );

        let mut linker = Linker::<ExecutionEnvironment>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::Io::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = Runtime::new(&mut store, &instance)?;
        if genesis {
            app.runtime_app().genesis(&mut store)?;
        }
        let mut apps = self.apps.write();
        let env = store.data();
        let changes = env.into();
        apps.insert(app_id, (app, store));
        Ok(Some(changes))
    }

    fn load_application(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address, //TODO; use codehash instead of app id
    ) -> anyhow::Result<()> {
        {
            let apps = self.apps.read();
            if apps.contains_key(&app_id) {
                return Ok(());
            }
        }
        let binary = state_db.get_app_source(app_id)?;
        let engine = &self.engine;
        let storage = state_db.get_app_data(app_id)?;
        let mut store = Store::new(
            engine,
            ExecutionEnvironment::new(
                Default::default(),
                app_id,
                0,
                storage,
                state_db.clone(),
                self.blockchain.clone(),
            )?,
        );

        let mut linker = Linker::<ExecutionEnvironment>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::Io::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, &binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = Runtime::new(&mut store, &instance)?;
        let mut apps = self.apps.write();
        apps.insert(app_id, (app, store));
        Ok(())
    }

    pub fn execute_call(
        &self,
        state_db: Arc<dyn StateDB>,
        origin: Address,
        call: &ApplicationCall,
        value: u64,
    ) -> anyhow::Result<Changelist> {
        self.load_application(state_db.clone(), call.app_id)?;

        let storage = state_db.get_app_data(call.app_id)?;
        let mut apps = self.apps.write();
        let (app, store) = apps
            .get_mut(&call.app_id)
            .ok_or_else(|| anyhow::anyhow!("app not found"))?;

        *store.data_mut() = ExecutionEnvironment::new(
            origin,
            call.app_id,
            value,
            storage,
            state_db.clone(),
            self.blockchain.clone(),
        )?;
        app.runtime_app().call(
            store.as_context_mut(),
            call.service,
            call.method,
            call.args.as_slice(),
        )?;
        let env = store.data();
        Ok(Changelist::from(env))
    }

    pub fn execute_query(
        &self,
        state_db: Arc<dyn StateDB>,
        call: &ApplicationCall,
    ) -> anyhow::Result<Vec<u8>> {
        let binary = state_db.get_app_source(call.app_id)?;
        let engine = &self.engine;
        let storage = state_db.get_app_data(call.app_id)?;
        let mut store = Store::new(engine, QueryEnvironment::new(storage, state_db.clone())?);

        let mut linker = Linker::<QueryEnvironment>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::Io::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, &binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = Runtime::new(&mut store, &instance)?;
        app.runtime_app()
            .query(store, call.service, call.method, call.args.as_slice())
    }

    pub fn execute_get_app_descriptor(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address,
    ) -> anyhow::Result<Vec<u8>> {
        self.load_application(state_db.clone(), app_id)?;
        let mut apps = self.apps.write();
        let (app, store) = apps
            .get_mut(&app_id)
            .ok_or_else(|| anyhow::anyhow!("app not loaded"))?;
        let app = app.runtime_app();
        app.descriptor(store)
    }
}

impl WasmVMInstance for WasmVM {
    fn execute_app_create<'a>(
        &self,
        state_db: Arc<dyn StateDB>,
        sender: Address,
        value: u64,
        call: &CreateApplication,
    ) -> anyhow::Result<(Vec<u8>, Changelist)> {
        let app_id = get_address_from_seed(
            call.package_name.as_bytes(),
            sender.network().ok_or_else(|| anyhow!("invalid network"))?,
        )?;
        self.create_application(state_db, sender, app_id, value, &call.binary)
    }

    fn execute_app_tx(
        &self,
        state_db: Arc<dyn StateDB>,
        sender: Address,
        value: u64,
        tx: &ApplicationCall,
    ) -> anyhow::Result<Changelist> {
        self.execute_call(state_db, sender, tx, value)
    }

    fn execute_app_query(
        &self,
        state_db: Arc<dyn StateDB>,
        call: &ApplicationCall,
    ) -> anyhow::Result<Vec<u8>> {
        self.execute_query(state_db, call)
    }

    fn execute_get_descriptor(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address,
    ) -> anyhow::Result<Vec<u8>> {
        self.execute_get_app_descriptor(state_db, app_id)
    }
}
