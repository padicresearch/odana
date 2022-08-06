use std::path::PathBuf;
use rt_wasm::RuntimeEngine;
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, value_parser)]
    wasm_file: PathBuf,

}

fn main() {
    let args = Args::parse();
    let mut rt = RuntimeEngine::new(args.wasm_file.as_path()).unwrap();
    println!("{:#?}", rt.execute("H", "Mar"));
}
