fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .extern_path(".odana.types", "::types::prelude")
        .extern_path(".odana.primitive_types", "::primitive_types")
        .compile(
            &[
                &"../proto/rpc_txs.proto".to_string(),
                &"../proto/rpc_account.proto".to_string(),
                &"../proto/rpc_chain.proto".to_string(),
                &"../proto/rpc_runtime.proto".to_string(),
                &"../proto/types.proto".to_string(),
            ],
            &[&"../proto".to_string()],
        )?;
    Ok(())
}
