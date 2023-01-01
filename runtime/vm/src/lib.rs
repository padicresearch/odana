use crate::env::Env;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use traits::{Blockchain, StateDB};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

mod env;

use crate::internal::{App, Context};
use types::account::{AccountState, Address42};

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/app.rs"));
}

struct WasmVM {
    engine: Engine,
    state_db: Arc<dyn StateDB>,
    blockchain: Arc<dyn Blockchain>,
    apps: Arc<RwLock<BTreeMap<u32, App>>>,
}

impl WasmVM {
    fn new(state_db: Arc<dyn StateDB>, blockchain: Arc<dyn Blockchain>) -> anyhow::Result<Self> {
        Engine::new(Config::new().consume_fuel(true)).map(|engine| Self {
            engine,
            state_db,
            blockchain,
            apps: Arc::new(Default::default()),
        })
    }

    pub fn instantiate_app(&self, app_id: u32, binary: Vec<u8>) -> anyhow::Result<()> {
        let engine = &self.engine;
        let mut store = Store::new(
            engine,
            Env::new(self.state_db.clone(), self.blockchain.clone()),
        );

        let mut linker = Linker::<Env>::new(engine);
        internal::blockchain_api::add_to_linker(&mut linker, |env| env)?;
        internal::balances_api::add_to_linker(&mut linker, |env| env)?;
        internal::storage::add_to_linker(&mut linker, |env| env)?;
        internal::event::add_to_linker(&mut linker, |env| env)?;

        let component = Component::from_binary(engine, &binary)?;
        let instance = linker.instantiate(&mut store, &component)?;

        let app = App::new(store, &instance)?;
        let mut apps = self.apps.write();
        apps.insert(app_id, app);
        Ok(())
    }

    pub fn execute_call(
        &self,
        app_id: u32,
        context: Context,
        call_arg: &[u8],
    ) -> anyhow::Result<HashMap<Address42, AccountState>> {
        let mut apps = self.apps.write();
        let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
        let mut store = Store::new(
            &self.engine,
            Env::new(self.state_db.clone(), self.blockchain.clone()),
        );
        app.call(&mut store, context, call_arg)?;
        let env = store.data();
        Ok(env.account_changes().clone())
    }

    pub fn execute_query(&self, app_id: u32, query: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut apps = self.apps.write();
        let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
        let mut store = Store::new(
            &self.engine,
            Env::new(self.state_db.clone(), self.blockchain.clone()),
        );
        app.query(&mut store, query)
    }
}
