use template::WASM_BINARY;

pub fn built_in_apps() -> Vec<(u32, &'static [u8])> {
    let mut apps = Vec::new();
    if let Some(template_wasm_binary) = WASM_BINARY {
        apps.push((0, template_wasm_binary));
    }
    apps
}
