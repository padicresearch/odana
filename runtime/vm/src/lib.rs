use crate::env::Env;
use parking_lot::RwLock;
use smt::SparseMerkleTree;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use traits::{Blockchain, ContextDB, StateDB};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

mod env;

use crate::internal::{App, Context};
use types::account::{AccountState, Address};

mod internal {
    include!(concat!(env!("OUT_DIR"), "/core.rs"));
    include!(concat!(env!("OUT_DIR"), "/io.rs"));
    include!(concat!(env!("OUT_DIR"), "/runtime.rs"));
}

// pub struct WasmVM {
//     engine: Arc<Engine>,
//     state_db: Arc<dyn StateDB>,
//     blockchain: Arc<dyn Blockchain>,
//     context_db: Arc<dyn ContextDB>,
//     apps: Arc<RwLock<BTreeMap<u32, App>>>,
// }
//
// impl WasmVM {
//     pub fn new(
//         state_db: Arc<dyn StateDB>,
//         blockchain: Arc<dyn Blockchain>,
//         context_db: Arc<dyn ContextDB>,
//     ) -> anyhow::Result<Self> {
//         Engine::new(Config::new().consume_fuel(true)).map(|engine| Self {
//             engine: Arc::new(engine),
//             state_db,
//             blockchain,
//             context_db,
//             apps: Arc::new(Default::default()),
//         })
//     }
//
//     pub fn instantiate_app(&self, app_id: u32, binary: Vec<u8>) -> anyhow::Result<()> {
//         let engine = &self.engine;
//         let raw_storage = self.context_db.app_state(app_id);
//         let storage: SparseMerkleTree = codec::Decodable::decode(raw_storage.as_ref())?;
//         let mut store = Store::new(
//             engine,
//             Env::new(storage, self.state_db.clone(), self.blockchain.clone()),
//         );
//
//         let mut linker = Linker::<Env>::new(engine);
//         internal::blockchain_api::add_to_linker(&mut linker, |env| env)?;
//         internal::balances_api::add_to_linker(&mut linker, |env| env)?;
//         internal::storage::add_to_linker(&mut linker, |env| env)?;
//         internal::event::add_to_linker(&mut linker, |env| env)?;
//
//         let component = Component::from_binary(engine, &binary)?;
//         let instance = linker.instantiate(&mut store, &component)?;
//
//         let app = App::new(store, &instance)?;
//         let mut apps = self.apps.write();
//         apps.insert(app_id, app);
//         Ok(())
//     }
//
//     pub fn execute_call(
//         &self,
//         app_id: u32,
//         context: Context,
//         call_arg: &[u8],
//     ) -> anyhow::Result<Changelist> {
//         let raw_storage = self.context_db.app_state(app_id);
//         let storage: SparseMerkleTree = codec::Decodable::decode(raw_storage.as_ref())?;
//         let mut apps = self.apps.write();
//         let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
//         let mut store = Store::new(
//             &self.engine,
//             Env::new(storage, self.state_db.clone(), self.blockchain.clone()),
//         );
//         app.call(&mut store, context, call_arg)?;
//         let env = store.into_data();
//         Ok(env.into())
//     }
//
//     pub fn execute_query(&self, app_id: u32, query: &[u8]) -> anyhow::Result<Vec<u8>> {
//         let raw_storage = self.context_db.app_state(app_id);
//         //TODO cache it
//         let storage: SparseMerkleTree = codec::Decodable::decode(raw_storage.as_ref())?;
//         let mut apps = self.apps.write();
//         let app = apps.get(&app_id).ok_or(anyhow::anyhow!("app not found"))?;
//         let mut store = Store::new(
//             &self.engine,
//             Env::new(storage, self.state_db.clone(), self.blockchain.clone()),
//         );
//         app.query(&mut store, query)
//     }
// }
