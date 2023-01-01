use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    wit_codegen::guest_generate(true, "../spec/io.wit", out_dir.as_str())?;
    Ok(())
}