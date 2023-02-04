use crate::env::ExecutionEnvironment;
use anyhow::anyhow;
use parking_lot::RwLock;
use primitive_types::Address;
use smt::SparseMerkleTree;
use std::collections::BTreeMap;
use std::sync::Arc;
use traits::{ChainHeadReader, StateDB, WasmVMInstance};
use wasmtime::component::{Component, Linker};
use wasmtime::{AsContextMut, Config, Engine, Store};

mod env;

use crate::internal::App;
use types::account::get_address_from_seed;
use types::prelude::{ApplicationCallTx, CreateApplicationTx};
use types::{Addressing, Changelist};

#[allow(clippy::all)]
#[allow(dead_code)]
mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}
type AppStateStore = BTreeMap<Address, (App, Store<ExecutionEnvironment>)>;
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
    ) -> anyhow::Result<Changelist> {
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
        internal::log::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::storage::add_to_linker(&mut linker, |env| env)?;
        internal::event::add_to_linker(&mut linker, |env| env)?;
        let component = Component::from_binary(engine, binary)?;
        let instance = linker.instantiate(&mut store, &component)?;
        let app = App::new(&mut store, &instance)?;
        let app = app.app();
        app.genesis(&mut store)?;
        let env = store.data();
        Ok(env.into())
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

        let app = App::new(&mut store, &instance)?;
        let mut apps = self.apps.write();
        apps.insert(app_id, (app, store));
        Ok(())
    }

    pub fn execute_call(
        &self,
        state_db: Arc<dyn StateDB>,
        origin: Address,
        app_id: Address,
        value: u64,
        call_arg: &[u8],
    ) -> anyhow::Result<Changelist> {
        self.load_application(state_db.clone(), app_id)?;

        let storage = state_db.get_app_data(app_id)?;
        let mut apps = self.apps.write();
        let (app, store) = apps
            .get_mut(&app_id)
            .ok_or_else(|| anyhow::anyhow!("app not found"))?;

        let app = app.app();
        *store.data_mut() = ExecutionEnvironment::new(
            origin,
            app_id,
            value,
            storage,
            state_db.clone(),
            self.blockchain.clone(),
        )?;
        app.call(store.as_context_mut(), call_arg)?;
        let env = store.data();
        Ok(Changelist::from(env))
    }

    pub fn execute_query(
        &self,
        state_db: Arc<dyn StateDB>,
        origin: Address,
        app_id: Address,
        value: u64,
        query: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>)> {
        self.load_application(state_db.clone(), app_id)?;
        let storage = state_db.get_app_data(app_id)?;

        println!("Loaded App At Root Hash {}", storage.root());
        let mut apps = self.apps.write();
        let (app, store) = apps
            .get_mut(&app_id)
            .ok_or_else(|| anyhow::anyhow!("app not loaded"))?;

        let app = app.app();
        *store.data_mut() = ExecutionEnvironment::new(
            origin,
            app_id,
            value,
            storage,
            state_db.clone(),
            self.blockchain.clone(),
        )?;
        let (n, res) = app.query(store, query)?;
        unsafe { Ok((String::from_utf8_unchecked(n), res)) }
    }
}

impl WasmVMInstance for WasmVM {
    fn execute_app_create<'a>(
        &self,
        state_db: Arc<dyn StateDB>,
        sender: Address,
        value: u64,
        call: &CreateApplicationTx,
    ) -> anyhow::Result<Changelist> {
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
        call: &ApplicationCallTx,
    ) -> anyhow::Result<Changelist> {
        self.execute_call(
            state_db,
            sender,
            Address::from_slice(&call.app_id).map_err(|_| anyhow!("invalid app address"))?,
            value,
            &call.args,
        )
    }

    fn execute_app_query(
        &self,
        state_db: Arc<dyn StateDB>,
        app_id: Address,
        raw_query: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>)> {
        self.execute_query(state_db, Address::default(), app_id, 0, raw_query)
    }
}
