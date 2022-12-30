use std::io::Result;
use wasm_builder::WasmBuilder;

fn main() -> Result<()> {
    let mut build = prost_build::Config::default();
    build.out_dir("src");
    build.compile_protos(&["proto/types.proto"], &["proto"])?;

    WasmBuilder::new()
        // Tell the builder to build the project (crate) this `build.rs` is part of.
        .with_current_project()
        // Make sure to export the `heap_base` global, this is required by Substrate
        .export_heap_base()
        // Build the Wasm file so that it imports the memory (need to be provided by at instantiation)
        .import_memory()
        // Build it.
        .build();

    Ok(())
}
