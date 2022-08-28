fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
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
