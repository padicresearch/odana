use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use wit_parser::World;

fn main() {
    wit_binding_generate("../spec/app.wit", "src");
    wit_binding_generate("../spec/runtime.wit", "src");
    wit_binding_generate("../spec/state.wit", "src");
    wit_binding_generate("../spec/storage.wit", "src");
}

fn wit_binding_generate(wit_file: &str, out_dir: &str) {
    let mut opts = wasmtime_wit_bindgen::Opts::default();
    opts.rustfmt = true;
    let world = World::parse_file(wit_file).unwrap();
    let content = opts.generate(&world);

    let mut out_file = PathBuf::new();
    out_file.push(out_dir);
    out_file.push(world.name);
    out_file.set_extension("rs");

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_file)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
}
