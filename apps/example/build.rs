use wasm_builder::WasmBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.out_dir("src");
    config.format(true);
    config.compile_protos(&[&"proto/types.proto".to_string()], &[&"proto".to_string()])?;

    WasmBuilder::new().with_current_project().build();

    Ok(())
}
