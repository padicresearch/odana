use wasm_builder::WasmBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.out_dir("src");
    config.format(true);

    let mut reflect_build = prost_reflect_build::Builder::new();
    reflect_build.compile_protos_with_config(
        config,
        &[&"proto/types.proto".to_string()],
        &[&"proto".to_string()],
    )?;

    WasmBuilder::new().with_current_project().build();

    Ok(())
}
