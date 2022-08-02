fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute("types.UnsignedTransaction", "#[derive(::serde::Serialize,::serde::Deserialize)]")
        .type_attribute("types.BlockHeader", "#[derive(::serde::Serialize,::serde::Deserialize)]")
        .compile(&["schema/rpc_account.proto", "schema/rpc_chain.proto", "schema/types.proto"], &["schema"])?;
    Ok(())
}