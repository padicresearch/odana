use odana_rt_api::Executable;
use template::{WASM_BINARY, metadata};

pub fn built_in_apps() -> Vec<(u32, &[u8])> {
    let mut apps = Vec::new();
    if let Some(template_wasm_binary) = WASM_BINARY {
        apps.push((0, template_wasm_binary);
    }
    apps
}