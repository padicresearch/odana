use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    wit_codegen::guest_generate(true, "../spec/core.wit", out_dir.as_str())?;
    wit_codegen::guest_generate(true, "../spec/app.wit", out_dir.as_str())?;
    Ok(())
}
