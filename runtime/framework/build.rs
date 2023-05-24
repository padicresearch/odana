use std::env;
use wit_codegen::Opts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();

    wit_codegen::guest_generate(
        Opts {
            rustfmt: true,
            unchecked: false,
            no_std: true,
            macro_export: true,
            ..Default::default()
        },
        "../spec/system.wit",
        out_dir.as_str(),
    )?;
    wit_codegen::guest_generate(
        Opts {
            macro_export: true,
            rustfmt: true,
            unchecked: false,
            no_std: true,
            export_macro_name: Some("export_app".to_string()),
            ..Default::default()
        },
        "../spec/runtime.wit",
        out_dir.as_str(),
    )?;
    wit_codegen::guest_generate(
        Opts {
            rustfmt: true,
            unchecked: false,
            no_std: true,
            macro_export: false,
            ..Default::default()
        },
        "../spec/io.wit",
        out_dir.as_str(),
    )?;
    Ok(())
}
