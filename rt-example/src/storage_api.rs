pub fn set(app_id: u32, key: &[u8], value: &[u8]) -> () {
    unsafe {
        let vec0 = key;
        let ptr0 = vec0.as_ptr() as i32;
        let len0 = vec0.len() as i32;
        let vec1 = value;
        let ptr1 = vec1.as_ptr() as i32;
        let len1 = vec1.len() as i32;
        #[link(wasm_import_module = "storage_api")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "set")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_api_set")]
            fn wit_import(_: i32, _: i32, _: i32, _: i32, _: i32);
        }
        wit_import(wit_bindgen_rust::rt::as_i32(app_id), ptr0, len0, ptr1, len1);
        ()
    }
}

pub fn get(app_id: u32, key: &[u8]) -> Vec<u8> {
    unsafe {
        let vec0 = key;
        let ptr0 = vec0.as_ptr() as i32;
        let len0 = vec0.len() as i32;
        let ptr1 = __STORAGE_API_RET_AREA.0.as_mut_ptr() as i32;
        #[link(wasm_import_module = "storage_api")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "get")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_api_get")]
            fn wit_import(_: i32, _: i32, _: i32, _: i32);
        }
        wit_import(wit_bindgen_rust::rt::as_i32(app_id), ptr0, len0, ptr1);
        let len2 = *((ptr1 + 4) as *const i32) as usize;
        Vec::from_raw_parts(*((ptr1 + 0) as *const i32) as *mut _, len2, len2)
    }
}

pub fn delete(app_id: u32, key: &[u8]) -> () {
    unsafe {
        let vec0 = key;
        let ptr0 = vec0.as_ptr() as i32;
        let len0 = vec0.len() as i32;
        #[link(wasm_import_module = "storage_api")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "delete")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_api_delete")]
            fn wit_import(_: i32, _: i32, _: i32);
        }
        wit_import(wit_bindgen_rust::rt::as_i32(app_id), ptr0, len0);
        ()
    }
}

#[repr(align(4))]
struct __StorageApiRetArea([u8; 8]);

static mut __STORAGE_API_RET_AREA: __StorageApiRetArea = __StorageApiRetArea([0; 8]);
