#[allow(clippy::all)]
pub mod storage {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;

    pub trait Storage: Sized {
        fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()>;
        fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>>;
        fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<bool>;
        fn root(&mut self) -> anyhow::Result<Vec<u8>>;
    }

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()>
    where
        U: Storage,
    {
        let mut inst = linker.instance("storage")?;
        inst.func_wrap(
            "insert",
            move |mut caller: wasmtime::StoreContextMut<'_, T>,
                  (arg0, arg1): (Vec<u8>, Vec<u8>)| {
                let host = get(caller.data_mut());
                let r = host.insert(arg0, arg1);
                r
            },
        )?;
        inst.func_wrap(
            "get",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Vec<u8>,)| {
                let host = get(caller.data_mut());
                let r = host.get(arg0);
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "remove",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Vec<u8>,)| {
                let host = get(caller.data_mut());
                let r = host.remove(arg0);
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "root",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (): ()| {
                let host = get(caller.data_mut());
                let r = host.root();
                Ok((r?,))
            },
        )?;
        Ok(())
    }
}

pub struct Storage {}
const _: () = {
    use wasmtime::component::__internal::anyhow;

    impl Storage {
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

            Ok(Storage {})
        }
    }
};
