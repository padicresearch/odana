use crate::env::Env;
use parking_lot::RwLock;
use primitive_types::{Address, H256};
use smt::SparseMerkleTree;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use traits::{AppData, Blockchain, StateDB};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

mod env;

use crate::internal::Runtime;
use types::account::AccountState;

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}

pub struct Changelist {
    pub account_changes: HashMap<Address, AccountState>,
    pub logs: Vec<Vec<u8>>,
    pub storage: SparseMerkleTree,
}

pub struct WasmVM {
    engine: Arc<Engine>,
    state_db: Arc<dyn StateDB>,
    appdata: Arc<dyn AppData>,
    blockchain: Arc<dyn Blockchain>,
    apps: Arc<RwLock<BTreeMap<Address, Runtime>>>,
}

impl WasmVM {
    pub fn new(
        state_db: Arc<dyn StateDB>,
        appdata: Arc<dyn AppData>,
        blockchain: Arc<dyn Blockchain>,
    ) -> anyhow::Result<Self> {
        Engine::new(Config::new().consume_fuel(true)).map(|engine| Self {
            engine: Arc::new(engine),
            state_db,
            appdata,
            blockchain,
            apps: Arc::new(Default::default()),
        })
    }

    pub fn instantiate_app(
        &self,
        app_id: Address,
        value: u64,
        binary: Vec<u8>,
    ) -> anyhow::Result<()> {
        let engine = &self.engine;
        let storage = self.appdata.get_app_data(app_id)?;
        let mut store = Store::new(
            engine,
            Env::new(
                app_id,
                value,
                storage,
                self.state_db.clone(),
                self.blockchain.clone(),
            )?,
        );

        let mut linker = Linker::<Env>::new(engine);
        internal::syscall::add_to_linker(&mut linker, |env| env)?;
        internal::log::add_to_linker(&mut linker, |env| env)?;
        internal::context::add_to_linker(&mut linker, |env| env)?;
        internal::storage::add_to_linker(&mut linker, |env| env)?;
        internal::event::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, &binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = Runtime::new(store, &instance)?;
        let mut apps = self.apps.write();
        apps.insert(app_id, app);
        Ok(())
    }

    pub fn execute_call(
        &self,
        app_id: Address,
        value: u64,
        call_arg: &[u8],
    ) -> anyhow::Result<Changelist> {
        let storage = self.appdata.get_app_data(app_id)?;
        let mut apps = self.apps.write();
        let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
        let mut store = Store::new(
            &self.engine,
            Env::new(
                app_id,
                value,
                storage,
                self.state_db.clone(),
                self.blockchain.clone(),
            )?,
        );
        app.app().call(&mut store, call_arg)?;
        let env = store.into_data();
        Ok(env.into())
    }

    pub fn execute_query(
        &self,
        app_id: Address,
        value: u64,
        query: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        let storage = self.appdata.get_app_data(app_id)?;
        let mut apps = self.apps.write();
        let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
        let mut store = Store::new(
            &self.engine,
            Env::new(
                app_id,
                value,
                storage,
                self.state_db.clone(),
                self.blockchain.clone(),
            ),
        );
        app.app().query(&mut store, query)
    }
}
