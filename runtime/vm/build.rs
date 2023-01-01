use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    wit_codegen::host_generate("../spec/core.wit", out_dir.as_str())?;
    wit_codegen::host_generate("../spec/app.wit", out_dir.as_str())?;
    wit_codegen::host_generate("../spec/io.wit", out_dir.as_str())?;
    Ok(())
}
