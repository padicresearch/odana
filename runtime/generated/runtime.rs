#[allow(clippy::all)]
pub mod runtime {
    #[allow(unused_imports)]
    use wit_bindgen_guest_rust::rt::{alloc, string::String, vec::Vec};

    pub fn on_event(event: &str, params: &[&str]) -> () {
        unsafe {
            let vec0 = event;
            let ptr0 = vec0.as_ptr() as i32;
            let len0 = vec0.len() as i32;
            let vec2 = params;
            let len2 = vec2.len() as i32;
            let layout2 = alloc::Layout::from_size_align_unchecked(vec2.len() * 8, 4);
            let result2 = if layout2.size() != 0 {
                let ptr = alloc::alloc(layout2);
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout2);
                }
                ptr
            } else {
                core::ptr::null_mut()
            };
            for (i, e) in vec2.into_iter().enumerate() {
                let base = result2 as i32 + (i as i32) * 8;
                {
                    let vec1 = e;
                    let ptr1 = vec1.as_ptr() as i32;
                    let len1 = vec1.len() as i32;
                    *((base + 4) as *mut i32) = len1;
                    *((base + 0) as *mut i32) = ptr1;
                }
            }

            #[link(wasm_import_module = "runtime")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "on-event")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "runtime_on-event")]
                fn wit_import(_: i32, _: i32, _: i32, _: i32);
            }
            wit_import(ptr0, len0, result2 as i32, len2);
            if layout2.size() != 0 {
                alloc::dealloc(result2, layout2);
            }
        }
    }
    pub fn finality_block_level() -> u32 {
        unsafe {
            #[link(wasm_import_module = "runtime")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "finality-block-level")]
                #[cfg_attr(
                    not(target_arch = "wasm32"),
                    link_name = "runtime_finality-block-level"
                )]
                fn wit_import() -> i32;
            }
            let ret = wit_import();
            ret as u32
        }
    }
    pub fn block_hash(level: u32) -> Vec<u8> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([u8; 8]);
            let mut ret_area = core::mem::MaybeUninit::<RetArea>::uninit();
            let ptr0 = ret_area.as_mut_ptr() as i32;

            #[link(wasm_import_module = "runtime")]
            extern "C" {
                #[cfg_attr(target_arch = "wasm32", link_name = "block-hash")]
                #[cfg_attr(not(target_arch = "wasm32"), link_name = "runtime_block-hash")]
                fn wit_import(_: i32, _: i32);
            }
            wit_import(wit_bindgen_guest_rust::rt::as_i32(level), ptr0);
            let len1 = *((ptr0 + 4) as *const i32) as usize;
            Vec::from_raw_parts(*((ptr0 + 0) as *const i32) as *mut _, len1, len1)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:runtime"]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 143] = [
    1, 0, 7, 114, 117, 110, 116, 105, 109, 101, 0, 97, 115, 109, 10, 0, 1, 0, 7, 109, 6, 112, 115,
    64, 2, 5, 101, 118, 101, 110, 116, 115, 6, 112, 97, 114, 97, 109, 115, 0, 1, 0, 64, 0, 0, 121,
    112, 125, 64, 1, 5, 108, 101, 118, 101, 108, 121, 0, 3, 66, 6, 2, 3, 2, 1, 1, 4, 8, 111, 110,
    45, 101, 118, 101, 110, 116, 0, 1, 0, 2, 3, 2, 1, 2, 4, 20, 102, 105, 110, 97, 108, 105, 116,
    121, 45, 98, 108, 111, 99, 107, 45, 108, 101, 118, 101, 108, 0, 1, 1, 2, 3, 2, 1, 4, 4, 10, 98,
    108, 111, 99, 107, 45, 104, 97, 115, 104, 0, 1, 2, 10, 12, 1, 7, 114, 117, 110, 116, 105, 109,
    101, 0, 5, 5,
];

#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}
