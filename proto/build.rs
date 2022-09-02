fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .extern_path(".uchain.types", "::types::prelude")
        .compile(
            &[
                "schema/rpc_txs.proto",
                "schema/rpc_account.proto",
                "schema/rpc_chain.proto",
            ],
            &["schema"],
        )?;
    Ok(())
}
