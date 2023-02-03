use crate::rpc::account_service_client::AccountServiceClient;
use crate::rpc::transactions_service_client::TransactionsServiceClient;
use crate::rpc::GetAccountRequest;
use crate::util::parse_cli_args_to_json;
use anyhow::{anyhow, bail};
use clap::{command, Args, Subcommand};
use pretty_hex::HexConfig;
use primitive_types::{Address, H256};
use prost::Message;
use protobuf::reflect::{FileDescriptor, MessageDescriptor};
use protobuf::text_format::print_to_string_pretty;
use protobuf_json_mapping::{Command, CommandError, ParseOptions};
use serde_json::{json, Number, Value};
use std::collections::HashMap;
use std::f32::consts::E;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use transaction::{make_payment_sign_transaction, make_signed_transaction};
use types::account::get_address_from_secret_key;
use types::network::Network;
use types::prelude::{get_address_from_seed, Empty};
use types::tx::{ApplicationCallTx, CreateApplicationTx, Transaction, TransactionData};

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
    App(AppArgsCommands),
    GetTxpool,
}

#[derive(Args, Debug)]
pub struct AppArgsCommands {
    #[clap(subcommand)]
    command: AppCommands,
}

#[derive(Args, Debug)]
pub struct SignerArgs {
    #[clap(long)]
    tip: u64,
    #[clap(long)]
    value: u64,
    #[clap(long, value_parser = parse_signer)]
    signer: H256,
}

#[derive(Args, Debug)]
pub struct ProtoFilesArg {
    #[clap(long)]
    proto_include: Vec<PathBuf>,
    #[clap(long)]
    proto_input: Vec<PathBuf>,
    #[clap(long)]
    schema_in: String,
    #[clap(long)]
    schema_out: Option<String>,
}

impl ProtoFilesArg {
    fn find_schemas(&self) -> anyhow::Result<(MessageDescriptor, Option<MessageDescriptor>)> {
        let mut schema_in = None;
        let mut schema_out = None;
        let files = protobuf_parse::Parser::new()
            .includes(
                self.proto_include
                    .iter()
                    .map(|f| std::fs::canonicalize(f.as_path()).unwrap()),
            )
            .inputs(std::fs::canonicalize(self.proto_input.as_path()).unwrap())
            .file_descriptor_set()?;
        'files_iter: for proto_file in files.file {
            let file_descriptor = FileDescriptor::new_dynamic(proto_file, &[])?;
            for message in file_descriptor.messages() {
                if schema_in.is_some() && schema_out.is_some() {
                    break 'files_iter;
                } else if schema_in.is_some() && self.schema_out.is_none() {
                    break 'files_iter;
                }
                if self.schema_in.eq(message.name()) {
                    schema_in = Some(message.clone());
                }

                if let Some(n) = &self.schema_out {
                    if n == message.name() {
                        schema_out = Some(message);
                    }
                }
            }
        }
        let Some(schema_in) = schema_in else {
            bail!("input schema [{}] message descriptor not found", self.schema_in)
        };
        Ok((schema_in, schema_out))
    }
}

#[derive(Subcommand, Debug)]
pub enum AppCommands {
    Call(CallArgs),
    Create(AppCreateArgs),
    Query(AppQueryArgs),
}

#[derive(Args, Debug)]
pub struct AddressArg {
    #[clap(long, value_parser = parse_address)]
    address: Address,
}

#[derive(Args, Debug)]
pub struct SendPaymentArgs {
    #[clap(long, value_parser = parse_address)]
    to: Address,
    #[clap(long)]
    amount: u64,
    #[clap(long)]
    fee: u64,
    #[clap(long, value_parser = parse_signer)]
    signer: H256,
}

#[derive(Args, Debug)]
pub struct CallArgs {
    #[clap(long)]
    app: String,
    #[clap(flatten)]
    sign_args: SignerArgs,
    #[clap(flatten)]
    proto: ProtoFilesArg,
    #[clap(require_equals = true, multiple = true)]
    params: Vec<String>,
}

#[derive(Args, Debug)]
pub struct AppCreateArgs {
    #[clap(long)]
    package_name: String,
    #[clap(flatten)]
    sign_args: SignerArgs,
    bin_path: PathBuf,
}

#[derive(Args, Debug)]
pub struct AppQueryArgs {
    #[clap(long)]
    package_name: String,
    #[clap(flatten)]
    proto: ProtoFilesArg,
    #[clap(require_equals = true, multiple = true)]
    params: Vec<String>,
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

pub fn handle_cmd_string(cmd: &Command) -> Result<Vec<u8>, CommandError> {
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

pub async fn handle_app_command(
    rpc_addr: &str,
    command: &AppArgsCommands,
) -> anyhow::Result<Value> {
    let mut account_service = AccountServiceClient::connect(format!("http://{}", rpc_addr)).await?;
    let mut tx_service = TransactionsServiceClient::connect(format!("http://{}", rpc_addr)).await?;

    let resp = match &command.command {
        AppCommands::Call(CallArgs {
                              app,
                              sign_args,
                              proto,
                              params,
                          }) => {
            let signer = sign_args.signer;
            let value = sign_args.value;
            let tip = sign_args.tip;

            let signer_address = get_address_from_secret_key(signer, Network::Testnet)?;

            let nonce = account_service
                .get_nonce(GetAccountRequest {
                    address: signer_address.to_vec(),
                })
                .await?
                .get_ref()
                .nonce;

            let app_id = get_address_from_seed(app.as_bytes(), Network::Testnet)?.to_vec();

            let opts = ParseOptions {
                ignore_unknown_fields: false,
                handler: &handle_cmd_string,
                _future_options: (),
            };

            let mut hex_config = HexConfig::default();
            hex_config.ascii = false;

            let json_value = parse_cli_args_to_json(params.iter())?;
            let json_string = serde_json::to_string(&json_value)?;

            let (call_message, _) = proto.find_schemas()?;
            let msg = protobuf_json_mapping::parse_dyn_from_str_with_options(
                &call_message,
                &json_string,
                &opts,
            )?;
            let mut encoded_call = Vec::new();
            msg.write_to_vec_dyn(&mut encoded_call)?;
            let data = TransactionData::Call(ApplicationCallTx {
                app_id,
                args: encoded_call,
            });
            let signed_tx =
                make_signed_transaction(signer, nonce, value, tip, Network::Testnet, data)?;
            let signed_tx_size = signed_tx.encode_to_vec().len();
            let response = tx_service.send_transaction(signed_tx).await?;
            json!({
                "tx_size" : signed_tx_size,
                "tx_hash" : H256::from_slice(&response.get_ref().hash),
            })
        }
        AppCommands::Create(AppCreateArgs {
                                package_name,
                                sign_args,
                                bin_path,
                            }) => {
            let signer = sign_args.signer;
            let value = sign_args.value;
            let tip = sign_args.tip;

            let signer_address = get_address_from_secret_key(signer, Network::Testnet)?;

            let nonce = account_service
                .get_nonce(GetAccountRequest {
                    address: signer_address.to_vec(),
                })
                .await?
                .get_ref()
                .nonce;

            let mut file = File::open(bin_path)?;
            let mut binary = Vec::with_capacity(file.metadata()?.len() as usize);
            let _ = file.read_to_end(&mut binary)?;
            let code_hash = crypto::keccak256(&binary);
            println!("Code Hash: {:?}", code_hash);
            let data = TransactionData::Create(CreateApplicationTx {
                package_name: package_name.to_owned(),
                binary,
            });
            let signed_tx =
                make_signed_transaction(signer, nonce, value, tip, Network::Testnet, data)?;
            let signed_tx_size = signed_tx.encode_to_vec().len();
            let response = tx_service.send_transaction(signed_tx).await?;
            json!({
                "code_hash" : code_hash,
                "tx_size" : signed_tx_size,
                "tx_hash" : H256::from_slice(&response.get_ref().hash),
            })
        }
        AppCommands::Query(AppQueryArgs {
                               package_name,
                               proto,
                               params,
                           }) => {
            json!({})
        }
    };
    Ok(resp)
}

pub async fn handle_client_command(command: &ClientArgsCommands) -> anyhow::Result<Value> {
    let resp = match &command.command {
        ClientCommands::GetBalance(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let balance = account_service
                .get_balance(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            Value::Number(balance.get_ref().balance.into())
        }
        ClientCommands::GetNonce(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let nonce = account_service
                .get_nonce(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            Value::Number(nonce.get_ref().nonce.into())
        }
        ClientCommands::GetAccountState(AddressArg { address }) => {
            let mut account_service =
                AccountServiceClient::connect(format!("http://{}", command.rpc_addr)).await?;
            let account_state = account_service
                .get_account_state(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            serde_json::to_value(account_state.get_ref())?
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

            let signed_tx_size = signed_tx.encode_to_vec().len();
            let response = tx_service.send_transaction(signed_tx).await?;

            json!({
                "tx_size" : signed_tx_size,
                "tx_hash" : H256::from_slice(&response.get_ref().hash),
            })
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
            json!({
                "queued" : queued_txs,
                "pending" : pending_txs,
            })
        }
        ClientCommands::App(a) => handle_app_command(&command.rpc_addr, a).await?,
    };
    Ok(resp)
}
