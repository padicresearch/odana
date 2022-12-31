#[allow(clippy::all)]
pub mod state {
    #[allow(unused_imports)]
    use wasmtime::component::__internal::anyhow;

    pub trait State: Sized {
        fn get_nonce(&mut self, address: Vec<u8>) -> anyhow::Result<u64>;
        fn get_free_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64>;
        fn get_reserve_balance(&mut self, address: Vec<u8>) -> anyhow::Result<u64>;
        fn add_free_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()>;
        fn sub_free_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()>;
        fn add_reserve_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()>;
        fn sub_reserve_balance(&mut self, address: Vec<u8>, amount: u64) -> anyhow::Result<()>;
    }

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::component::Linker<T>,
        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()>
    where
        U: State,
    {
        let mut inst = linker.instance("state")?;
        inst.func_wrap(
            "get-nonce",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Vec<u8>,)| {
                let host = get(caller.data_mut());
                let r = host.get_nonce(arg0);
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "get-free-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Vec<u8>,)| {
                let host = get(caller.data_mut());
                let r = host.get_free_balance(arg0);
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "get-reserve-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0,): (Vec<u8>,)| {
                let host = get(caller.data_mut());
                let r = host.get_reserve_balance(arg0);
                Ok((r?,))
            },
        )?;
        inst.func_wrap(
            "add-free-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0, arg1): (Vec<u8>, u64)| {
                let host = get(caller.data_mut());
                let r = host.add_free_balance(arg0, arg1);
                r
            },
        )?;
        inst.func_wrap(
            "sub-free-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0, arg1): (Vec<u8>, u64)| {
                let host = get(caller.data_mut());
                let r = host.sub_free_balance(arg0, arg1);
                r
            },
        )?;
        inst.func_wrap(
            "add-reserve-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0, arg1): (Vec<u8>, u64)| {
                let host = get(caller.data_mut());
                let r = host.add_reserve_balance(arg0, arg1);
                r
            },
        )?;
        inst.func_wrap(
            "sub-reserve-balance",
            move |mut caller: wasmtime::StoreContextMut<'_, T>, (arg0, arg1): (Vec<u8>, u64)| {
                let host = get(caller.data_mut());
                let r = host.sub_reserve_balance(arg0, arg1);
                r
            },
        )?;
        Ok(())
    }
}

pub struct State {}
const _: () = {
    use wasmtime::component::__internal::anyhow;

    impl State {
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

            Ok(State {})
        }
    }
};
