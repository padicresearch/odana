use crate::env::ExecutionEnvironment;
use anyhow::anyhow;
use parking_lot::RwLock;
use primitive_types::{Address, H256};
use smt::SparseMerkleTree;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use traits::{AppData, Blockchain, ChainHeadReader, StateDB, WasmVMInstance};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

mod env;

use crate::internal::App;
use types::account::{get_address_from_seed, AccountState};
use types::prelude::{ApplicationCallTx, CreateApplicationTx};
use types::{Addressing, Changelist};

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}

pub struct WasmVM {
    engine: Arc<Engine>,
    appdata: Arc<dyn AppData>,
    blockchain: Arc<dyn ChainHeadReader>,
    apps: Arc<RwLock<BTreeMap<Address, App>>>,
}

impl WasmVM {
    pub fn new(
        appdata: Arc<dyn AppData>,
        blockchain: Arc<dyn ChainHeadReader>,
    ) -> anyhow::Result<Self> {
        Engine::new(Config::new().consume_fuel(true)).map(|engine| Self {
            engine: Arc::new(engine),
            appdata,
            blockchain,
            apps: Arc::new(Default::default()),
        })
    }

    pub fn create_application<'a>(
        &self,
        state_db: &'a dyn StateDB,
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
        let env = store.into_data();
        Ok(env.into())
    }

    pub fn load_application<'a>(
        &self,
        state_db: &'a dyn StateDB,
        origin: Address,
        app_id: Address,
        value: u64,
        binary: Vec<u8>,
    ) -> anyhow::Result<()> {
        let engine = &self.engine;
        let storage = self.appdata.get_app_data(app_id)?;
        let mut store = Store::new(
            engine,
            ExecutionEnvironment::new(
                origin,
                app_id,
                value,
                storage,
                state_db.clone(),
                self.blockchain.clone(),
            )?,
        );

        let mut linker = Linker::<ExecutionEnvironment>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::log::add_to_linker(&mut linker, |env| env)?;
        internal::execution_context::add_to_linker(&mut linker, |env| env)?;
        internal::storage::add_to_linker(&mut linker, |env| env)?;
        internal::event::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, &binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = App::new(store, &instance)?;
        let mut apps = self.apps.write();
        apps.insert(app_id, app);
        Ok(())
    }

    pub fn execute_call<'a>(
        &self,
        state_db: &'a dyn StateDB,
        origin: Address,
        app_id: Address,
        value: u64,
        call_arg: &[u8],
    ) -> anyhow::Result<Changelist> {
        let storage = self.appdata.get_app_data(app_id)?;
        let mut apps = self.apps.write();
        let app = apps
            .get(&app_id)
            .ok_or(anyhow::anyhow!("app not found"))?
            .app();
        let mut store = Store::new(
            &self.engine,
            ExecutionEnvironment::new(
                origin,
                app_id,
                value,
                storage,
                state_db.clone(),
                self.blockchain.clone(),
            )?,
        );
        app.call(&mut store, call_arg)?;
        let env = store.into_data();
        Ok(env.into())
    }

    pub fn execute_query<'a>(
        &self,
        state_db: &'a dyn StateDB,
        origin: Address,
        app_id: Address,
        value: u64,
        query: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        let storage = self.appdata.get_app_data(app_id)?;
        let mut apps = self.apps.write();
        let app = apps
            .get(&app_id)
            .ok_or(anyhow::anyhow!("app not found"))?
            .app();
        let mut store = Store::new(
            &self.engine,
            ExecutionEnvironment::new(
                origin,
                app_id,
                value,
                storage,
                state_db.clone(),
                self.blockchain.clone(),
            ),
        );
        app.query(&mut store, query)
    }
}

impl WasmVMInstance for WasmVM {
    fn execute_app_create<'a>(
        &self,
        state_db: &'a dyn StateDB,
        sender: Address,
        value: u64,
        call: &CreateApplicationTx,
    ) -> anyhow::Result<Changelist> {
        let app_id = get_address_from_seed(
            call.package_name.as_bytes(),
            sender.network().ok_or(anyhow!("invalid network"))?,
        )?;
        self.create_application(state_db, sender, app_id, value, &call.binary)
    }

    fn execute_app_tx<'a>(
        &self,
        state_db: &'a dyn StateDB,
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

    fn execute_app_query<'a>(
        &self,
        state_db: &'a dyn StateDB,
        app_id: Address,
        raw_query: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        self.execute_query(state_db, Address::default(), app_id, 0, raw_query)
    }
}
