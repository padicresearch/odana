use std::path::Path;
use std::sync::Arc;
use wit_bindgen_wasmtime::wasmtime::{Engine, Instance, Linker, Module, Store};
use wit_bindgen_wasmtime::anyhow::Result;
use crate::rt::{Runtime, RuntimeData};
use crate::storage::StorageBackend;

mod storage;
mod rt;

pub struct RuntimeEngine {
    rt: Runtime<RuntimeData>,
    instance: Instance,
    store: Store<RuntimeData>,
}


impl RuntimeEngine {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self> {
        let storage = Arc::new(StorageBackend);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        storage::add_to_linker(&mut linker, storage)?;
        let module = Module::from_file(&engine, file)?;
        let state = RuntimeData {};
        let mut store = Store::new(&engine, state);
        let (rt, instance) = Runtime::instantiate(&mut store, &module, &mut linker)?;
        Ok(Self {
            rt,
            instance,
            store,
        })
    }
    pub fn execute(
        &mut self,
        ctx: &str,
        raw_tx: &str,
    ) -> Result<String> {
        self.rt.execute(&mut self.store, ctx, raw_tx).map_err(|e| e.into())
    }
}
