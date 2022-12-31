#[derive(wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone)]
pub struct Context<'a> {
    #[component(name = "block-level")]
    pub block_level: u32,
    #[component(name = "chain-id")]
    pub chain_id: u32,
    #[component(name = "miner")]
    pub miner: &'a [u8],
    #[component(name = "sender")]
    pub sender: &'a [u8],
    #[component(name = "fee")]
    pub fee: u64,
}
impl<'a> core::fmt::Debug for Context<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("block-level", &self.block_level)
            .field("chain-id", &self.chain_id)
            .field("miner", &self.miner)
            .field("sender", &self.sender)
            .field("fee", &self.fee)
            .finish()
    }
}
pub struct App {
    call: wasmtime::component::Func,
    init: wasmtime::component::Func,
    query: wasmtime::component::Func,
}
const _: () = {
    use wasmtime::component::__internal::anyhow;

    impl App {
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

            let call = *__exports
                .typed_func::<(Context<'_>, &[u8]), ()>("call")?
                .func();
            let init = *__exports.typed_func::<(u32, &[u8]), (u32,)>("init")?.func();
            let query = *__exports
                .typed_func::<(&[u8],), (Vec<u8>,)>("query")?
                .func();
            Ok(App { call, init, query })
        }
        pub fn init<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
            arg0: u32,
            arg1: &[u8],
        ) -> anyhow::Result<u32> {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(u32, &[u8]), (u32,)>::new_unchecked(self.init)
            };
            let (ret0,) = callee.call(store.as_context_mut(), (arg0, arg1))?;
            callee.post_return(store.as_context_mut())?;
            Ok(ret0)
        }
        pub fn call<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
            arg0: Context<'_>,
            arg1: &[u8],
        ) -> anyhow::Result<()> {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(Context<'_>, &[u8]), ()>::new_unchecked(self.call)
            };
            let () = callee.call(store.as_context_mut(), (arg0, arg1))?;
            callee.post_return(store.as_context_mut())?;
            Ok(())
        }
        pub fn query<S: wasmtime::AsContextMut>(
            &self,
            mut store: S,
            arg0: &[u8],
        ) -> anyhow::Result<Vec<u8>> {
            let callee = unsafe {
                wasmtime::component::TypedFunc::<(&[u8],), (Vec<u8>,)>::new_unchecked(self.query)
            };
            let (ret0,) = callee.call(store.as_context_mut(), (arg0,))?;
            callee.post_return(store.as_context_mut())?;
            Ok(ret0)
        }
    }
};
