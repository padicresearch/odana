use std::sync::Arc;
#[allow(unused_imports)]
use wit_bindgen_wasmtime::{anyhow, wasmtime};

pub(crate) trait StorageApi: Sync + Send {
    fn set(&self, app_id: u32, key: &[u8], value: &[u8]) -> ();

    fn get(&self, app_id: u32, key: &[u8]) -> Vec<u8>;

    fn delete(&self, app_id: u32, key: &[u8]) -> ();
}

pub(crate) fn add_to_linker<T>(
    linker: &mut wasmtime::Linker<T>,
    host: Arc<dyn StorageApi>,
) -> anyhow::Result<()>
{
    use wit_bindgen_wasmtime::rt::get_func;
    use wit_bindgen_wasmtime::rt::get_memory;
    {
        let host = host.clone();
        linker.func_wrap(
            "storage_api",
            "set",
            move |mut caller: wasmtime::Caller<'_, T>,
                  arg0: i32,
                  arg1: i32,
                  arg2: i32,
                  arg3: i32,
                  arg4: i32| {
                let memory = &get_memory(&mut caller, "memory")?;
                let (mem, data) = memory.data_and_store_mut(&mut caller);
                let mut _bc = wit_bindgen_wasmtime::BorrowChecker::new(mem);
                let ptr0 = arg1;
                let len0 = arg2;
                let ptr1 = arg3;
                let len1 = arg4;
                let param0 = arg0 as u32;
                let param1 = _bc.slice(ptr0, len0)?;
                let param2 = _bc.slice(ptr1, len1)?;
                let result = host.set(param0, param1, param2);
                let () = result;
                Ok(())
            },
        )?;
    }
    {
        let host = host.clone();
        linker.func_wrap(
            "storage_api",
            "get",
            move |mut caller: wasmtime::Caller<'_, T>, arg0: i32, arg1: i32, arg2: i32, arg3: i32| {
                let func = get_func(&mut caller, "canonical_abi_realloc")?;
                let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
                let memory = &get_memory(&mut caller, "memory")?;
                let (mem, data) = memory.data_and_store_mut(&mut caller);
                let mut _bc = wit_bindgen_wasmtime::BorrowChecker::new(mem);
                let ptr0 = arg1;
                let len0 = arg2;
                let param0 = arg0 as u32;
                let param1 = _bc.slice(ptr0, len0)?;
                let result = host.get(param0, param1);
                let vec1 = result;
                let ptr1 =
                    func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, (vec1.len() as i32) * 1))?;
                let caller_memory = memory.data_mut(&mut caller);
                caller_memory.store_many(ptr1, &vec1)?;
                caller_memory.store(
                    arg3 + 4,
                    wit_bindgen_wasmtime::rt::as_i32(vec1.len() as i32),
                )?;
                caller_memory.store(arg3 + 0, wit_bindgen_wasmtime::rt::as_i32(ptr1))?;
                Ok(())
            },
        )?;
    }

    linker.func_wrap(
        "storage_api",
        "delete",
        move |mut caller: wasmtime::Caller<'_, T>, arg0: i32, arg1: i32, arg2: i32| {
            let memory = &get_memory(&mut caller, "memory")?;
            let (mem, data) = memory.data_and_store_mut(&mut caller);
            let mut _bc = wit_bindgen_wasmtime::BorrowChecker::new(mem);
            let ptr0 = arg1;
            let len0 = arg2;
            let param0 = arg0 as u32;
            let param1 = _bc.slice(ptr0, len0)?;
            let result = host.delete(param0, param1);
            let () = result;
            Ok(())
        },
    )?;
    Ok(())
}

use wit_bindgen_wasmtime::rt::RawMem;

pub(crate) struct StorageBackend;

impl StorageApi for StorageBackend {
    fn set(&self, app_id: u32, key: &[u8], value: &[u8]) -> () {
        println!("app_id {app_id}, {:?} {:?}", key, value);
    }

    fn get(&self, app_id: u32, key: &[u8]) -> Vec<u8> {
        println!("app_id {app_id}, {:?}", key);
        Vec::new()
    }

    fn delete(&self, app_id: u32, key: &[u8]) -> () {
        println!("app_id {app_id}, {:?}", key);
    }
}
