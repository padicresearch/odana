use std::path::PathBuf;
use wit_bindgen_core::Files;
use wit_parser::World;

pub fn guest_generate(macro_export: bool, wit_file: &str, out_dir: &str) -> anyhow::Result<()> {
    let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
    opts.rustfmt = true;
    opts.no_std = true;
    opts.unchecked = false;
    opts.macro_export = macro_export;

    let mut files = Files::default();
    let mut generator = opts.build();
    let world = World::parse_file(wit_file)?;
    generator.generate(&world, &mut files);
    let mut path = PathBuf::new();
    path.push(out_dir);

    for (name, contents) in files.iter() {
        let dst = path.join(name);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dst, contents)?;
    }
    Ok(())
}

pub fn host_generate(wit_file: &str, out_dir: &str) -> anyhow::Result<()> {
    let mut opts = wasmtime_wit_bindgen::Opts::default();
    opts.rustfmt = true;
    let world = World::parse_file(wit_file).unwrap();
    let contents = opts.generate(&world);
    let contents = contents.as_bytes();

    let mut dst = PathBuf::new();
    dst.push(out_dir);
    dst.push(world.name);
    dst.set_extension("rs");

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dst, contents)?;
    Ok(())
}
