fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .type_attribute(
            "types.UnsignedTransaction",
            "#[derive(::serde::Serialize,::serde::Deserialize)]",
        )
        .type_attribute(
            "types.Transaction",
            "#[derive(::serde::Serialize,::serde::Deserialize)]",
        )
        .type_attribute(
            "types.BlockHeader",
            "#[derive(::serde::Serialize,::serde::Deserialize)]",
        ).type_attribute(
        "types.Block",
        "#[derive(::serde::Serialize,::serde::Deserialize)]",
    )
        .type_attribute(
            "types.AccountState",
            "#[derive(::serde::Serialize,::serde::Deserialize)]",
        ).type_attribute(
        "types.TransactionList",
        "#[derive(::serde::Serialize,::serde::Deserialize)]",
    ).type_attribute(
        "types.TransactionStatus",
        "#[derive(::serde::Serialize,::serde::Deserialize)]",
    ).type_attribute(
        "types.TransactionStatus",
        "#[serde(rename_all = \"lowercase\")]",
    )
        .compile(
            &[
                "schema/types.proto",
                "schema/rpc_txs.proto",
                "schema/rpc_account.proto",
                "schema/rpc_chain.proto",
            ],
            &["schema"],
        )?;
    Ok(())
}


