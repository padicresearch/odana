use anyhow::{anyhow, bail};
use std::path::PathBuf;
use wit_bindgen_core::Files;
use wit_parser::{Resolve, UnresolvedPackage, World};

pub fn guest_generate(macro_export: bool, wit_file: &str, out_dir: &str) -> anyhow::Result<()> {
    let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
    opts.rustfmt = true;
    opts.no_std = true;
    opts.unchecked = false;
    opts.macro_export = macro_export;

    let mut resolve = Resolve::default();
    let mut files = Files::default();
    let mut generator = opts.build();

    let pkg = resolve.push(
        UnresolvedPackage::parse_file(wit_file.as_ref())?,
        &Default::default(),
    )?;

    let mut docs = resolve.packages[pkg].documents.iter();
    let (_, doc) = docs
        .next()
        .ok_or_else(|| anyhow!("no documents found in package"))?;
    if docs.next().is_some() {
        bail!("multiple documents found in package, specify which to bind with `--world` argument")
    }
    let world = resolve.documents[*doc]
        .default_world
        .ok_or_else(|| anyhow!("no default world in document"))?;

    generator.generate(&resolve, world, &mut files);
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
    let mut resolve = Resolve::default();
    let mut opts = wasmtime_wit_bindgen::Opts::default();
    opts.rustfmt = true;

    let pkg = resolve.push(
        UnresolvedPackage::parse_file(wit_file.as_ref())?,
        &Default::default(),
    )?;

    let mut docs = resolve.packages[pkg].documents.iter();
    let (_, doc) = docs
        .next()
        .ok_or_else(|| anyhow!("no documents found in package"))?;
    if docs.next().is_some() {
        bail!("multiple documents found in package, specify which to bind with `--world` argument")
    }
    let world = resolve.documents[*doc]
        .default_world
        .ok_or_else(|| anyhow!("no default world in document"))?;

    let name = &resolve.documents[*doc].name;

    let contents = opts.generate(&resolve, world);
    let contents = contents.as_bytes();

    let mut dst = PathBuf::new();
    dst.push(out_dir);
    dst.push(name);
    dst.set_extension("rs");

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dst, contents)?;
    Ok(())
}
