use odana_rt_api::Executable;
use template::{WASM_BINARY, metadata};

pub fn built_in_apps() -> Vec<(u32, Executable)> {
    let mut apps = Vec::new();
    if let Some(template_wasm_binary) = WASM_BINARY {
        apps.push((0, Executable {
            binary: template_wasm_binary.to_vec(),
            metadata: metadata(),
        }))
    }
    apps
}