#![no_std]

use prost::Message;
use odana_std::prelude::*;

use odana_std::prelude::*;
#[derive(Clone)]
pub struct ExecutionContext {
    pub block_level: u32,
    pub chain_id: u32,
    pub miner: Vec<u8>,
    pub sender: Vec<u8>,
    pub fee: u64,
}
impl core::fmt::Debug for ExecutionContext {
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

pub mod app {
    use super::*;

    #[doc(hidden)]
    pub unsafe fn call_init<T: RuntimeApplication>(arg0: i32, arg1: i32, arg2: i32) -> i32 {
        let len0 = arg2 as usize;
        let genesis = Vec::from_raw_parts(arg1 as *mut _, len0, len0);
        let genesis = Message::decode(genesis.as_slice()).expect("failed to decode args");
        let result1 = T::init(arg0 as u32, genesis);
        wit_bindgen_guest_rust::rt::as_i32(result1)
    }

    #[doc(hidden)]
    pub unsafe fn call_call<T: RuntimeApplication>(
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
        arg4: i32,
        arg5: i32,
        arg6: i64,
        arg7: i32,
        arg8: i32,
    ) {
        let len0 = arg3 as usize;
        let len1 = arg5 as usize;
        let len2 = arg8 as usize;

        let call = Vec::from_raw_parts(arg7 as *mut _, len2, len2);
        let call  = Message::decode(call.as_slice()).expect("failed to decode args");
        T::call(
            ExecutionContext {
                block_level: arg0 as u32,
                chain_id: arg1 as u32,
                miner: Vec::from_raw_parts(arg2 as *mut _, len0, len0),
                sender: Vec::from_raw_parts(arg4 as *mut _, len1, len1),
                fee: arg6 as u64,
            },
            call,
        );
    }

    #[doc(hidden)]
    pub unsafe fn call_query<T: RuntimeApplication>(arg0: i32, arg1: i32) -> i32 {
        let len0 = arg1 as usize;
        let query = Vec::from_raw_parts(arg0 as *mut _, len0, len0);
        let query = T::Query::decode(query.as_slice()).expect("failed to decode args");
        let result1 = T::query(query).encode_to_vec();
        let ptr2 = RET_AREA.0.as_mut_ptr() as i32;
        let vec3 = (result1).into_boxed_slice();
        let ptr3 = vec3.as_ptr() as i32;
        let len3 = vec3.len() as i32;
        core::mem::forget(vec3);
        *((ptr2 + 4) as *mut i32) = len3;
        *((ptr2 + 0) as *mut i32) = ptr3;
        ptr2
    }

    #[doc(hidden)]
    pub unsafe fn post_return_query<T: RuntimeApplication>(arg0: i32) {
        let base0 = *((arg0 + 0) as *const i32);
        let len0 = *((arg0 + 4) as *const i32);
        wit_bindgen_guest_rust::rt::dealloc(base0, (len0 as usize) * 1, 1);
    }

    #[repr(align(4))]
    struct AppRetArea([u8; 8]);
    static mut RET_AREA: AppRetArea = AppRetArea([0; 8]);
}

pub trait RuntimeApplication{
    type Genesis: prost::Message + Default;
    type Call: prost::Message + Default;
    type Query: prost::Message + Default;
    type QueryResponse: prost::Message + Default;

    fn init(block_level: u32, genesis: Self::Genesis) -> u32;
    fn call(context: ExecutionContext, call: Self::Call);
    fn query(query: Self::Query) -> Self::QueryResponse;
}


/// Declares the export of the component's world for the
/// given type.
#[macro_export]
macro_rules! export_app(($t:ident) => {
  const _: () = {

    #[doc(hidden)]
    #[export_name = "app#init"]
    unsafe extern "C" fn __export_app_init(arg0: i32,arg1: i32,arg2: i32,) -> i32 {
      app::call_init::<$t>(arg0,arg1,arg2,)
    }

    #[doc(hidden)]
    #[export_name = "app#call"]
    unsafe extern "C" fn __export_app_call(arg0: i32,arg1: i32,arg2: i32,arg3: i32,arg4: i32,arg5: i32,arg6: i64,arg7: i32,arg8: i32,) {
      app::call_call::<$t>(arg0,arg1,arg2,arg3,arg4,arg5,arg6,arg7,arg8,)
    }

    #[doc(hidden)]
    #[export_name = "app#query"]
    unsafe extern "C" fn __export_app_query(arg0: i32,arg1: i32,) -> i32 {
      app::call_query::<$t>(arg0,arg1,)
    }

    #[doc(hidden)]
    #[export_name = "cabi_post_app#query"]
    unsafe extern "C" fn __post_return_app_query(arg0: i32,) {
      app::post_return_query::<$t>(arg0,)
    }

  };

  #[used]
  #[doc(hidden)]
  #[cfg(target_arch = "wasm32")]
  static __FORCE_SECTION_REF: fn() = __force_section_ref;
  #[doc(hidden)]
  #[cfg(target_arch = "wasm32")]
  fn __force_section_ref() {
    __link_section()
  }
});

#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:app"]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 179] = [
    1, 0, 0, 97, 115, 109, 10, 0, 1, 0, 7, 157, 1, 6, 112, 125, 64, 2, 11, 98, 108, 111, 99, 107,
    45, 108, 101, 118, 101, 108, 121, 7, 103, 101, 110, 101, 115, 105, 115, 0, 0, 121, 114, 5, 11,
    98, 108, 111, 99, 107, 45, 108, 101, 118, 101, 108, 121, 8, 99, 104, 97, 105, 110, 45, 105,
    100, 121, 5, 109, 105, 110, 101, 114, 0, 6, 115, 101, 110, 100, 101, 114, 0, 3, 102, 101, 101,
    119, 64, 2, 1, 99, 2, 4, 99, 97, 108, 108, 0, 1, 0, 64, 1, 5, 113, 117, 101, 114, 121, 0, 0, 0,
    66, 8, 2, 3, 2, 1, 2, 4, 7, 99, 111, 110, 116, 101, 120, 116, 3, 0, 0, 2, 3, 2, 1, 1, 4, 4,
    105, 110, 105, 116, 1, 1, 2, 3, 2, 1, 3, 4, 4, 99, 97, 108, 108, 1, 2, 2, 3, 2, 1, 4, 4, 5,
    113, 117, 101, 114, 121, 1, 3, 11, 7, 1, 3, 97, 112, 112, 3, 5,
];

#[inline(never)]
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub fn __link_section() {}