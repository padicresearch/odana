#[allow(clippy::all)]
pub mod state {
    #[allow(unused_imports)]
    use wit_bindgen_guest_rust::rt::{alloc, string::String, vec::Vec};

    pub fn get_nonce(address: &[u8]) -> u64 {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get-nonce")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_get-nonce")]
                fn wit_import(_: i32, _: i32) -> i64;
            }
            let ret = wit_import(ptr0, len0);
            ret as u64
        }
    }
    pub fn get_free_balance(address: &[u8]) -> u64 {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get-free-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_get-free-balance")]
                fn wit_import(_: i32, _: i32) -> i64;
            }
            let ret = wit_import(ptr0, len0);
            ret as u64
        }
    }
    pub fn get_reserve_balance(address: &[u8]) -> u64 {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "get-reserve-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_get-reserve-balance")]
                fn wit_import(_: i32, _: i32) -> i64;
            }
            let ret = wit_import(ptr0, len0);
            ret as u64
        }
    }
    pub fn add_free_balance(address: &[u8], amount: u64) -> () {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "add-free-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_add-free-balance")]
                fn wit_import(_: i32, _: i32, _: i64);
            }
            wit_import(ptr0, len0, wit_bindgen_guest_rust::rt::as_i64(amount));
        }
    }
    pub fn sub_free_balance(address: &[u8], amount: u64) -> () {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "sub-free-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_sub-free-balance")]
                fn wit_import(_: i32, _: i32, _: i64);
            }
            wit_import(ptr0, len0, wit_bindgen_guest_rust::rt::as_i64(amount));
        }
    }
    pub fn add_reserve_balance(address: &[u8], amount: u64) -> () {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "add-reserve-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_add-reserve-balance")]
                fn wit_import(_: i32, _: i32, _: i64);
            }
            wit_import(ptr0, len0, wit_bindgen_guest_rust::rt::as_i64(amount));
        }
    }
    pub fn sub_reserve_balance(address: &[u8], amount: u64) -> () {
        unsafe {
            let vec0 = address;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;

            #[link(wasm_import_module = "state")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "sub-reserve-balance")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "state_sub-reserve-balance")]
                fn wit_import(_: i32, _: i32, _: i64);
            }
            wit_import(ptr0, len0, wit_bindgen_guest_rust::rt::as_i64(amount));
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:state"]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 229] = [
    1, 0, 5, 115, 116, 97, 116, 101, 0, 97, 115, 109, 10, 0, 1, 0, 7, 198, 1, 4, 112, 125, 64, 1,
    7, 97, 100, 100, 114, 101, 115, 115, 0, 0, 119, 64, 2, 7, 97, 100, 100, 114, 101, 115, 115, 0,
    6, 97, 109, 111, 117, 110, 116, 119, 1, 0, 66, 9, 2, 3, 2, 1, 1, 4, 9, 103, 101, 116, 45, 110,
    111, 110, 99, 101, 0, 1, 0, 4, 16, 103, 101, 116, 45, 102, 114, 101, 101, 45, 98, 97, 108, 97,
    110, 99, 101, 0, 1, 0, 4, 19, 103, 101, 116, 45, 114, 101, 115, 101, 114, 118, 101, 45, 98, 97,
    108, 97, 110, 99, 101, 0, 1, 0, 2, 3, 2, 1, 2, 4, 16, 97, 100, 100, 45, 102, 114, 101, 101, 45,
    98, 97, 108, 97, 110, 99, 101, 0, 1, 1, 4, 16, 115, 117, 98, 45, 102, 114, 101, 101, 45, 98,
    97, 108, 97, 110, 99, 101, 0, 1, 1, 4, 19, 97, 100, 100, 45, 114, 101, 115, 101, 114, 118, 101,
    45, 98, 97, 108, 97, 110, 99, 101, 0, 1, 1, 4, 19, 115, 117, 98, 45, 114, 101, 115, 101, 114,
    118, 101, 45, 98, 97, 108, 97, 110, 99, 101, 0, 1, 1, 10, 10, 1, 5, 115, 116, 97, 116, 101, 0,
    5, 3,
];

#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}
