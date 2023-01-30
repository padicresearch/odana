use std::env::set_var;
use wasm_builder::WasmBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.out_dir("src");
    config.compile_protos(&[&format!("proto/types.proto")], &[&format!("proto")])?;
    //
    WasmBuilder::new()
        .with_current_project()
        .export_heap_base()
        .import_memory()
        .build();

    Ok(())
}
