fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute("txs.UnsignedTransaction", "#[derive(Debug)]")
        .compile(&["schema/blockchain.proto", "schema/txs.proto"], &["schema"])?;
    Ok(())
}