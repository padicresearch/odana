use crate::rpc::account_service_client::AccountServiceClient;
use crate::rpc::transactions_service_client::TransactionsServiceClient;
use crate::rpc::GetAccountRequest;
use clap::{Args, Subcommand};
use primitive_types::H256;
use std::collections::HashMap;
use std::str::FromStr;
use transaction::make_payment_sign_transaction;
use types::account::{get_address_from_secret_key, Address};
use types::network::Network;
use types::prelude::Empty;

#[derive(Args, Debug)]
pub struct ClientArgsCommands {
    #[clap(subcommand)]
    command: ClientCommands,
    #[clap(long, default_value_t = String::from("127.0.0.1:9121"))]
    rpc_addr: String,
}

#[derive(Subcommand, Debug)]
pub enum ClientCommands {
    GetBalance(AddressArg),
    GetNonce(AddressArg),
    GetAccountState(AddressArg),
    SendPayment(SendPaymentArgs),
    GetTxpool,
}

#[derive(Args, Debug)]
pub struct AddressArg {
    #[clap(short, long, value_parser = parse_address)]
    address: Address,
}

#[derive(Args, Debug)]
pub struct SendPaymentArgs {
    #[clap(short, long, value_parser = parse_address)]
    to: Address,
    #[clap(short, long)]
    amount: u64,
    #[clap(short, long)]
    fee: u64,
    #[clap(short, long, value_parser = parse_signer)]
    signer: H256,
}

pub(crate) fn parse_address(s: &str) -> Result<Address, String> {
    if s.eq_ignore_ascii_case("ama")
        || s.eq_ignore_ascii_case("kofi")
        || s.eq_ignore_ascii_case("kwame")
    {
        return Ok(account::create_account_from_uri(Network::Testnet, s).address);
    }
    match Address::from_str(s) {
        Ok(s) => Ok(s),
        Err(error) => Err(format!("{}", error)),
    }
}

pub(crate) fn parse_signer(s: &str) -> Result<H256, String> {
    if s.eq_ignore_ascii_case("ama")
        || s.eq_ignore_ascii_case("kofi")
        || s.eq_ignore_ascii_case("kwame")
    {
        return Ok(account::create_account_from_uri(Network::Testnet, s).secret);
    }
    hex::decode(s)
        .map_err(|e| format!("{}", e))
        .map(|decode_hex| H256::from_slice(&decode_hex))
}

pub async fn handle_client_command(command: &ClientArgsCommands) -> anyhow::Result<()> {
    match &command.command {
        ClientCommands::GetBalance(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let balance = account_service
                .get_balance(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            println!("{}", balance.get_ref().balance)
        }
        ClientCommands::GetNonce(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let nonce = account_service
                .get_nonce(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            println!("{}", nonce.get_ref().nonce)
        }
        ClientCommands::GetAccountState(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let account_state = account_service
                .get_account_state(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;

            println!(
                "{}",
                serde_json::to_string_pretty(account_state.get_ref()).unwrap_or_default()
            )
        }
        ClientCommands::SendPayment(SendPaymentArgs {
            to,
            amount,
            signer,
            fee,
        }) => {
            // Todo get network
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;

            let signer_address = get_address_from_secret_key(*signer, Network::Testnet)?;

            let nonce = account_service
                .get_nonce(GetAccountRequest {
                    address: signer_address.to_vec(),
                })
                .await?
                .get_ref()
                .nonce;
            let mut tx_service =
                TransactionsServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let signed_tx = make_payment_sign_transaction(
                *signer,
                *to,
                nonce,
                *amount,
                *fee,
                Network::Testnet,
            )?;
            let response = tx_service.send_transaction(signed_tx).await?;
            println!("{}", hex::encode(&response.get_ref().hash, false))
        }
        ClientCommands::GetTxpool => {
            let mut tx_service =
                TransactionsServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let txpool_content = tx_service.get_txpool_content(Empty).await?;

            let queued_txs: HashMap<_, _> = txpool_content
                .get_ref()
                .queued
                .iter()
                .map(|r| {
                    (
                        Address::from_slice(&r.address).unwrap_or_default(),
                        &r.txs,
                    )
                })
                .collect();

            let pending_txs: HashMap<_, _> = txpool_content
                .get_ref()
                .pending
                .iter()
                .map(|r| {
                    (
                        Address::from_slice(&r.address).unwrap_or_default(),
                        &r.txs,
                    )
                })
                .collect();

            // TODO: use table in pretty mode
            let json_rep = serde_json::json!({
                "queued" : queued_txs,
                "pending" : pending_txs,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&json_rep).unwrap_or_default()
            )
        }
    }

    Ok(())
}
