use std::env::set_var;
use wasm_builder::WasmBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.out_dir("src");
    config.format(true);
    config.compile_protos(&[&format!("proto/types.proto")], &[&format!("proto")])?;

    WasmBuilder::new().with_current_project().build();

    Ok(())
}
