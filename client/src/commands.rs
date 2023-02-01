use crate::rpc::account_service_client::AccountServiceClient;
use crate::rpc::transactions_service_client::TransactionsServiceClient;
use crate::rpc::GetAccountRequest;
use crate::util::parse_cli_args_to_json;
use anyhow::{anyhow, bail};
use clap::{Args, Subcommand};
use pretty_hex::HexConfig;
use primitive_types::{Address, H256};
use protobuf::reflect::FileDescriptor;
use protobuf::text_format::print_to_string_pretty;
use protobuf_json_mapping::{Command, CommandError, ParseOptions};
use std::collections::HashMap;
use std::f32::consts::E;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use transaction::make_payment_sign_transaction;
use types::account::get_address_from_secret_key;
use types::network::Network;
use types::prelude::Empty;
use types::tx::Transaction;

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
    Call(CallArgs),
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

#[derive(Args, Debug)]
pub struct CallArgs {
    #[clap(short, long)]
    app: String,
    #[clap(short, long)]
    proto_include: Vec<PathBuf>,
    #[clap(short, long)]
    proto_input: PathBuf,
    #[clap(short, long)]
    message: String,
    #[clap(short, long)]
    tip: u64,
    #[clap(short, long)]
    value: u64,

    #[clap(require_equals = true, multiple = true)]
    call_args: Vec<String>,
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

pub fn handle_command(cmd: &Command) -> Result<Vec<u8>, CommandError> {
    match cmd.op {
        "hex" => {
            hex::decode(cmd.data).map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))
        }
        "address" => parse_address(cmd.data)
            .map(|addr| addr.to_vec())
            .map_err(|e| CommandError::FailedToParseNom(format!("{}", e))),

        "file" => {
            let file_path = PathBuf::new().join(cmd.data);
            if !file_path.is_file() {
                return Err(CommandError::FailedToParseNom(format!("path not file")));
            }
            let mut file = File::open(file_path.as_path())
                .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))?;
            let mut out = Vec::with_capacity(
                file.metadata()
                    .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))?
                    .len() as usize,
            );
            let _read_len = file
                .read_to_end(&mut out)
                .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)));
            Ok(out)
        }
        _ => Err(CommandError::FailedToParse),
    }
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
                .map(|r| (Address::from_slice(&r.address).unwrap_or_default(), &r.txs))
                .collect();

            let pending_txs: HashMap<_, _> = txpool_content
                .get_ref()
                .pending
                .iter()
                .map(|r| (Address::from_slice(&r.address).unwrap_or_default(), &r.txs))
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
        ClientCommands::Call(CallArgs {
                                 app,
                                 tip,
                                 value,
                                 message,
                                 proto_include,
                                 proto_input,
                                 call_args,
                             }) => {
            let opts = ParseOptions {
                ignore_unknown_fields: false,
                handler: &handle_command,
                _future_options: (),
            };

            let mut hex_config = HexConfig::default();
            hex_config.ascii = false;

            let json_value = parse_cli_args_to_json(call_args.iter())?;
            let json_string = serde_json::to_string(&json_value)?;

            let files = protobuf_parse::Parser::new()
                .includes(
                    proto_include
                        .iter()
                        .map(|f| std::fs::canonicalize(f.as_path()).unwrap()),
                )
                .input(std::fs::canonicalize(proto_input.as_path()).unwrap())
                .file_descriptor_set()?;
            let proto_file = files.file[0].clone();
            let file_descriptor = FileDescriptor::new_dynamic(proto_file, &[])?;
            let call_message = file_descriptor
                .message_by_package_relative_name(message)
                .ok_or(anyhow!("message: {message} not found"))?;
            let msg = protobuf_json_mapping::parse_dyn_from_str_with_options(
                &call_message,
                &json_string,
                &opts,
            )?;
            println!("Call: {}", print_to_string_pretty(msg.as_ref()));
            let mut encoded_message = Vec::new();
            msg.write_to_vec_dyn(&mut encoded_message)?;
            println!("Call Encoded");
            println!("{}", pretty_hex::config_hex(&encoded_message, hex_config));
        }
    }

    Ok(())
}
