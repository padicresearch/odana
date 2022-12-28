fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .build_client(false)
        .extern_path(".uchain.types", "::types::prelude")
        .compile(
            &[
                &format!("../proto/rpc_txs.proto"),
                &format!("../proto/rpc_account.proto"),
                &format!("../proto/rpc_chain.proto"),
                &format!("../proto/types.proto"),
            ],
            &[&format!("../proto")],
        )?;
    Ok(())
}
