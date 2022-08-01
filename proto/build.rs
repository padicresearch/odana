fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute("tx.UnsignedTransaction", "#[derive(::serde::Serialize,::serde::Deserialize)]")
        .type_attribute("blockchain.BlockHeader", "#[derive(::serde::Serialize,::serde::Deserialize)]")
        .compile(&["schema/blockchain.proto", "schema/txs.proto"], &["schema"])?;
    Ok(())
}