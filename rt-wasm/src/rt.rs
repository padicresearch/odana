use std::marker::PhantomData;
#[allow(unused_imports)]
use wit_bindgen_wasmtime::{anyhow, wasmtime};

/// Auxiliary data associated with the wasm exports.
///
/// This is required to be stored within the data of a
/// `Store<T>` itself so lifting/lowering state can be managed
/// when translating between the host and wasm.
#[derive(Default)]
pub struct RuntimeData {}

pub struct Runtime<T> {
    canonical_abi_free: wasmtime::TypedFunc<(i32, i32, i32), ()>,
    canonical_abi_realloc: wasmtime::TypedFunc<(i32, i32, i32, i32), i32>,
    execute: wasmtime::TypedFunc<(i32, i32, i32, i32), (i32, )>,
    memory: wasmtime::Memory,
    rpc: wasmtime::TypedFunc<(i32, i32, i32, i32), (i32, )>,
    _data: PhantomData<T>,
}

impl<T> Runtime<T> {
    #[allow(unused_variables)]
    /// Adds any intrinsics, if necessary for this exported wasm
    /// functionality to the `linker` provided.
    pub fn add_to_linker(
        linker: &mut wasmtime::Linker<T>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Instantiates the provided `module` using the specified
    /// parameters, wrapping up the result in a structure that
    /// translates between wasm and the host.
    ///
    /// The `linker` provided will have intrinsics added to it
    /// automatically, so it's not necessary to call
    /// `add_to_linker` beforehand. This function will
    /// instantiate the `module` otherwise using `linker`, and
    /// both an instance of this structure and the underlying
    /// `wasmtime::Instance` will be returned.
    ///
    /// The `get_state` parameter is used to access the
    /// auxiliary state necessary for these wasm exports from
    /// the general store state `T`.
    pub fn instantiate(
        mut store: impl wasmtime::AsContextMut<Data=T>,
        module: &wasmtime::Module,
        linker: &mut wasmtime::Linker<T>,
    ) -> anyhow::Result<(Self, wasmtime::Instance)> {
        Self::add_to_linker(linker)?;
        let instance = linker.instantiate(&mut store, module)?;
        Ok((Self::new(store, &instance)?, instance))
    }

    /// Low-level creation wrapper for wrapping up the exports
    /// of the `instance` provided in this structure of wasm
    /// exports.
    ///
    /// This function will extract exports from the `instance`
    /// defined within `store` and wrap them all up in the
    /// returned structure which can be used to interact with
    /// the wasm module.
    pub fn new(
        mut store: impl wasmtime::AsContextMut<Data=T>,
        instance: &wasmtime::Instance,
    ) -> anyhow::Result<Self> {
        let mut store = store.as_context_mut();
        let canonical_abi_free =
            instance.get_typed_func::<(i32, i32, i32), (), _>(&mut store, "canonical_abi_free")?;
        let canonical_abi_realloc = instance
            .get_typed_func::<(i32, i32, i32, i32), i32, _>(&mut store, "canonical_abi_realloc")?;
        let execute =
            instance.get_typed_func::<(i32, i32, i32, i32), (i32, ), _>(&mut store, "execute")?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("`memory` export not a memory"))?;
        let rpc = instance.get_typed_func::<(i32, i32, i32, i32), (i32, ), _>(&mut store, "rpc")?;
        Ok(Runtime {
            canonical_abi_free,
            canonical_abi_realloc,
            execute,
            memory,
            rpc,
            _data: Default::default(),
        })
    }
    pub fn execute(
        &self,
        mut caller: impl wasmtime::AsContextMut<Data=T>,
        ctx: &str,
        raw_tx: &str,
    ) -> Result<String, wasmtime::Trap> {
        let func_canonical_abi_free = &self.canonical_abi_free;
        let func_canonical_abi_realloc = &self.canonical_abi_realloc;
        let memory = &self.memory;
        let vec0 = ctx;
        let ptr0 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
        memory
            .data_mut(&mut caller)
            .store_many(ptr0, vec0.as_bytes())?;
        let vec1 = raw_tx;
        let ptr1 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec1.len() as i32))?;
        memory
            .data_mut(&mut caller)
            .store_many(ptr1, vec1.as_bytes())?;
        let (result2_0, ) = self.execute.call(
            &mut caller,
            (ptr0, vec0.len() as i32, ptr1, vec1.len() as i32),
        )?;
        let load3 = memory.data_mut(&mut caller).load::<i32>(result2_0 + 0)?;
        let load4 = memory.data_mut(&mut caller).load::<i32>(result2_0 + 4)?;
        let ptr5 = load3;
        let len5 = load4;

        let data5 = copy_slice(&mut caller, memory, ptr5, len5, 1)?;
        func_canonical_abi_free.call(&mut caller, (ptr5, len5, 1))?;
        Ok(String::from_utf8(data5).map_err(|_| wasmtime::Trap::new("invalid utf-8"))?)
    }
    pub fn rpc(
        &self,
        mut caller: impl wasmtime::AsContextMut<Data=T>,
        ctx: &str,
        raw_rpc: &str,
    ) -> Result<String, wasmtime::Trap> {
        let func_canonical_abi_realloc = &self.canonical_abi_realloc;
        let func_canonical_abi_free = &self.canonical_abi_free;
        let memory = &self.memory;
        let vec0 = ctx;
        let ptr0 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
        memory
            .data_mut(&mut caller)
            .store_many(ptr0, vec0.as_bytes())?;
        let vec1 = raw_rpc;
        let ptr1 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec1.len() as i32))?;
        memory
            .data_mut(&mut caller)
            .store_many(ptr1, vec1.as_bytes())?;
        let (result2_0, ) = self.rpc.call(
            &mut caller,
            (ptr0, vec0.len() as i32, ptr1, vec1.len() as i32),
        )?;
        let load3 = memory.data_mut(&mut caller).load::<i32>(result2_0 + 0)?;
        let load4 = memory.data_mut(&mut caller).load::<i32>(result2_0 + 4)?;
        let ptr5 = load3;
        let len5 = load4;

        let data5 = copy_slice(&mut caller, memory, ptr5, len5, 1)?;
        func_canonical_abi_free.call(&mut caller, (ptr5, len5, 1))?;
        Ok(String::from_utf8(data5).map_err(|_| wasmtime::Trap::new("invalid utf-8"))?)
    }
}

use wit_bindgen_wasmtime::rt::copy_slice;
use wit_bindgen_wasmtime::rt::RawMem;
