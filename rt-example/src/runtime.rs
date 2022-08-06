#[export_name = "execute"]
unsafe extern "C" fn __wit_bindgen_runtime_execute(
    arg0: i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
) -> i32 {
    let len0 = arg1 as usize;
    let len1 = arg3 as usize;
    let result = <super::Runtime as Runtime>::execute(
        String::from_utf8(Vec::from_raw_parts(arg0 as *mut _, len0, len0)).unwrap(),
        String::from_utf8(Vec::from_raw_parts(arg2 as *mut _, len1, len1)).unwrap(),
    );
    let ptr2 = __RUNTIME_RET_AREA.0.as_mut_ptr() as i32;
    let vec3 = (result.into_bytes()).into_boxed_slice();
    let ptr3 = vec3.as_ptr() as i32;
    let len3 = vec3.len() as i32;
    core::mem::forget(vec3);
    *((ptr2 + 4) as *mut i32) = len3;
    *((ptr2 + 0) as *mut i32) = ptr3;
    ptr2
}

#[export_name = "rpc"]
unsafe extern "C" fn __wit_bindgen_runtime_rpc(arg0: i32, arg1: i32, arg2: i32, arg3: i32) -> i32 {
    let len0 = arg1 as usize;
    let len1 = arg3 as usize;
    let result = <super::Runtime as Runtime>::rpc(
        String::from_utf8(Vec::from_raw_parts(arg0 as *mut _, len0, len0)).unwrap(),
        String::from_utf8(Vec::from_raw_parts(arg2 as *mut _, len1, len1)).unwrap(),
    );
    let ptr2 = __RUNTIME_RET_AREA.0.as_mut_ptr() as i32;
    let vec3 = (result.into_bytes()).into_boxed_slice();
    let ptr3 = vec3.as_ptr() as i32;
    let len3 = vec3.len() as i32;
    core::mem::forget(vec3);
    *((ptr2 + 4) as *mut i32) = len3;
    *((ptr2 + 0) as *mut i32) = ptr3;
    ptr2
}

#[repr(align(4))]
struct __RuntimeRetArea([u8; 8]);

static mut __RUNTIME_RET_AREA: __RuntimeRetArea = __RuntimeRetArea([0; 8]);

pub trait Runtime {
    fn execute(ctx: String, raw_tx: String) -> String;
    fn rpc(ctx: String, raw_rpc: String) -> String;
}
