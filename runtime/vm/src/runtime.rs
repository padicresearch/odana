#[allow(clippy::all)]
pub mod runtime {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;

    pub trait Runtime: Sized {
        fn on_event(&mut self, event: String, params: Vec<String>) -> anyhow::Result<()>;
        fn finality_block_level(&mut self) -> anyhow::Result<u32>;
        fn block_hash(&mut self, level: u32) -> anyhow::Result<Vec<u8>>;
    }

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()>
    where
        U: Runtime,
    {
        let mut inst = linker.instance("runtime")?;
        inst.func_wrap(
            "on-event",
            move |mut caller: wasmtime::StoreContextMut<'_, T>,
                  (arg0, arg1): (String, Vec<String>)| {
                let host = get(caller.data_mut());
                let r = host.on_event(arg0, arg1);
                r
            },
        )?;
        inst.func_wrap(
            "finality-block-level",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                let host = get(caller.data_mut());
                let r = host.finality_block_level();
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "block-hash",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (u32,)| {
                let host = get(caller.data_mut());
                let r = host.block_hash(arg0);
                Ok((r?,))
            },
        )?;
        Ok(())
    }
}

pub struct Runtime {}
const _: () = {
    use wasmtime::component::__internal::anyhow;

    impl Runtime {
        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        pub fn instantiate<T>(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            component: &wasmtime::component::Component,
            linker: &wasmtime::component::Linker<T>,
        ) -> anyhow::Result<(Self, wasmtime::component::Instance)> {
            let instance = linker.instantiate(&mut store, component)?;
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
            mut store: impl wasmtime::AsContextMut,
            instance: &wasmtime::component::Instance,
        ) -> anyhow::Result<Self> {
            let mut store = store.as_context_mut();
            let mut exports = instance.exports(&mut store);
            let mut __exports = exports.root();

            Ok(Runtime {})
        }
    }
};
