#[allow(clippy::all)]
pub mod storage {
    #[allow(unused_imports)]
    use wit_bindgen_guest_rust::rt::{alloc, string::String, vec::Vec};

    pub fn insert(key: &[u8], value: &[u8]) -> () {
        unsafe {
            let vec0 = key;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let vec1 = value;
            let ptr1 = vec1.as_ptr() as i32;
            let len1 = vec1.len() as i32;
            #[link(wasm_import_module = "storage")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "insert")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_insert")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32);
            }
            wit_import(ptr0, len0, ptr1, len1);
        }
    }
    pub fn get(key: &[u8]) -> Option<Vec<u8>> {
        unsafe {
            let vec0 = key;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[repr(align(4))]
            struct RetArea([u8; 12]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr1 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "storage")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_get")]
                fn wit_import(_: i32, _: i32, _: i32);
            }
            wit_import(ptr0, len0, ptr1);
            match i32::from(*((ptr1 + 0) as *const u8)) {
                0 => None,
                1 => Some({
                    let len2 = *((ptr1 + 8) as *const i32) as usize;

                    Vec::from_raw_parts(*((ptr1 + 4) as *const i32) as *mut _, len2, len2)
                }),
                _ => panic!("invalid enum discriminant"),
            }
        }
    }
    pub fn remove(key: &[u8]) -> bool {
        unsafe {
            let vec0 = key;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            #[link(wasm_import_module = "storage")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "remove")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_remove")]
                fn wit_import(_: i32, _: i32) -> i32;
            }
            let ret = wit_import(ptr0, len0);
            match ret {
                0 => false,
                1 => true,
                _ => panic!("invalid bool discriminant"),
            }
        }
    }
    pub fn root() -> Vec<u8> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;
            #[link(wasm_import_module = "storage")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "root")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "storage_root")]
                fn wit_import(_: i32);
            }
            wit_import(ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            Vec::from_raw_parts(*((ptr0 + 0) as *const i32) as *mut _, len1, len1)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:storage"]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 125] = [
    1, 0, 0, 97, 115, 109, 10, 0, 1, 0, 7, 100, 7, 112, 125, 64, 2, 3, 107, 101, 121, 0, 5, 118,
    97, 108, 117, 101, 0, 1, 0, 107, 0, 64, 1, 3, 107, 101, 121, 0, 0, 2, 64, 1, 3, 107, 101, 121,
    0, 0, 127, 64, 0, 0, 0, 66, 8, 2, 3, 2, 1, 1, 4, 6, 105, 110, 115, 101, 114, 116, 1, 0, 2, 3,
    2, 1, 3, 4, 3, 103, 101, 116, 1, 1, 2, 3, 2, 1, 4, 4, 6, 114, 101, 109, 111, 118, 101, 1, 2, 2,
    3, 2, 1, 5, 4, 4, 114, 111, 111, 116, 1, 3, 10, 11, 1, 7, 115, 116, 111, 114, 97, 103, 101, 5,
    6,
];

#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}
