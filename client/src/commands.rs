use crate::rpc::{GetAccountRequest, Query};
use crate::util::parse_cli_args_to_json;
use crate::Client;
use anyhow::bail;
use clap::{Args, Subcommand};
use primitive_types::{Address, H256};
use prost::Message;
use protobuf::descriptor::FileDescriptorSet;
use protobuf::reflect::{FileDescriptor, MessageDescriptor};

use protobuf_json_mapping::{Command, CommandError, ParseOptions};
use serde_json::{json, Value};
use std::collections::HashMap;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use transaction::{make_payment_sign_transaction, make_signed_transaction};
use types::account::get_address_from_secret_key;
use types::network::Network;
use types::prelude::{get_address_from_seed, Empty};
use types::tx::{ApplicationCallTx, CreateApplicationTx, TransactionData};

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
    schema: String,
}

impl ProtoFilesArg {
    fn schema(&self) -> anyhow::Result<MessageDescriptor> {
        let mut schema_in = None;
        let files = self.get_file_description_sets()?;
        'files_iter: for proto_file in files.file {
            let file_descriptor = FileDescriptor::new_dynamic(proto_file, &[])?;
            //file_descriptor.message_by_full_name()
            for message in file_descriptor.messages() {
                if self.schema.eq(message.full_name()) {
                    schema_in = Some(message);
                    break 'files_iter;
                }
            }
        }
        let Some(schema_in) = schema_in else {
            bail!("input schema [{}] message descriptor not found", self.schema)
        };
        Ok(schema_in)
    }

    fn get_file_description_sets(&self) -> anyhow::Result<FileDescriptorSet> {
        protobuf_parse::Parser::new()
            .includes(
                self.proto_include
                    .iter()
                    .map(|f| std::fs::canonicalize(f.as_path()).unwrap()),
            )
            .inputs(
                self.proto_input
                    .iter()
                    .map(|f| std::fs::canonicalize(f.as_path()).unwrap()),
            )
            .file_descriptor_set()
    }

    fn schemas(&self) -> anyhow::Result<(MessageDescriptor, HashMap<String, MessageDescriptor>)> {
        let mut schemas = HashMap::new();
        let files = self.get_file_description_sets()?;
        for proto_file in files.file {
            let file_descriptor = FileDescriptor::new_dynamic(proto_file, &[])?;
            for message in file_descriptor.messages() {
                schemas.insert(message.full_name().to_string(), message);
            }
        }
        let Some(schema_in) = schemas.get(&self.schema) else {
            bail!("input schema [{}] message descriptor not found", self.schema)
        };
        Ok((schema_in.clone(), schemas))
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
    app: String,
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
            .map_err(CommandError::FailedToParseNom),

        "file" => {
            let file_path = PathBuf::new().join(cmd.data);
            if !file_path.is_file() {
                return Err(CommandError::FailedToParseNom("path not file".to_string()));
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
    rpc_client: &Client,
    command: &AppArgsCommands,
) -> anyhow::Result<Value> {
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

            let nonce = rpc_client
                .account_service()
                .get_nonce(GetAccountRequest {
                    address: signer_address.to_vec(),
                })
                .await?
                .get_ref()
                .nonce;

            let app_id = get_address_from_seed(app.as_bytes(), Network::Testnet)?;
            let app_id = app_id.to_vec();

            let opts = ParseOptions {
                ignore_unknown_fields: false,
                handler: &handle_cmd_string,
                _future_options: (),
            };

            let json_value = parse_cli_args_to_json(params.iter())?;
            let json_string = serde_json::to_string(&json_value)?;

            let schema_in = proto.schema()?;
            let msg = protobuf_json_mapping::parse_dyn_from_str_with_options(
                &schema_in,
                &json_string,
                &opts,
            )?;

            let call_json: Value = serde_json::from_str(
                &protobuf_json_mapping::print_to_string(msg.as_ref())
                    .expect("failed to print message as json"),
            )?;

            let mut encoded_call = Vec::new();
            msg.write_to_vec_dyn(&mut encoded_call)?;
            let data = TransactionData::Call(ApplicationCallTx {
                app_id,
                args: encoded_call,
            });
            let signed_tx =
                make_signed_transaction(signer, nonce, value, tip, Network::Testnet, data)?;
            let signed_tx_size = signed_tx.encoded_len();
            let response = rpc_client
                .transaction_service()
                .send_transaction(signed_tx)
                .await?;

            json!({
                "call" : call_json,
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

            let nonce = rpc_client
                .account_service()
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
            let data = TransactionData::Create(CreateApplicationTx {
                package_name: package_name.to_owned(),
                binary,
            });
            let signed_tx =
                make_signed_transaction(signer, nonce, value, tip, Network::Testnet, data)?;
            let signed_tx_size = signed_tx.encoded_len();
            let response = rpc_client
                .transaction_service()
                .send_transaction(signed_tx)
                .await?;
            json!({
                "code_hash" : code_hash,
                "tx_size" : signed_tx_size,
                "tx_hash" : H256::from_slice(&response.get_ref().hash),
            })
        }
        AppCommands::Query(AppQueryArgs { app, proto, params }) => {
            let (schema_in, schemas) = proto.schemas()?;

            let opts = ParseOptions {
                ignore_unknown_fields: false,
                handler: &handle_cmd_string,
                _future_options: (),
            };

            let app_id = get_address_from_seed(app.as_bytes(), Network::Testnet)?;
            let app_id = app_id.to_vec();
            let json_value = parse_cli_args_to_json(params.iter())?;
            let json_string = serde_json::to_string(&json_value)?;
            let query_message = protobuf_json_mapping::parse_dyn_from_str_with_options(
                &schema_in,
                &json_string,
                &opts,
            )?;
            let mut query = Vec::new();
            query_message.write_to_vec_dyn(&mut query)?;
            let response = rpc_client
                .runtime_api_service()
                .query_runtime(Query { app_id, query })
                .await?;

            let Some(response_message_desc) = schemas.get(response.get_ref().typename.as_str()) else {
                let raw = hex::encode_raw(response.get_ref().data.as_slice());
                return Ok(
                    json!({
                    "raw" : raw
                }))
            };

            let mut response_message = response_message_desc.new_instance();
            response_message.merge_from_bytes_dyn(response.get_ref().data.as_slice())?;

            serde_json::from_str(&protobuf_json_mapping::print_to_string(
                response_message.as_ref(),
            )?)?
        }
    };
    Ok(resp)
}

pub async fn handle_client_command(command: &ClientArgsCommands) -> anyhow::Result<Value> {
    let rpc_client = Client::connect(format!("http://{}", command.rpc_addr)).await?;

    let resp = match &command.command {
        ClientCommands::GetBalance(AddressArg { address }) => {
            let balance = rpc_client
                .account_service()
                .get_balance(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            Value::Number(balance.get_ref().balance.into())
        }
        ClientCommands::GetNonce(AddressArg { address }) => {
            let nonce = rpc_client
                .account_service()
                .get_nonce(GetAccountRequest {
                    address: address.to_vec(),
                })
                .await?;
            Value::Number(nonce.get_ref().nonce.into())
        }
        ClientCommands::GetAccountState(AddressArg { address }) => {
            let account_state = rpc_client
                .account_service()
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

            let signer_address = get_address_from_secret_key(*signer, Network::Testnet)?;

            let nonce = rpc_client
                .account_service()
                .get_nonce(GetAccountRequest {
                    address: signer_address.to_vec(),
                })
                .await?
                .get_ref()
                .nonce;

            let signed_tx = make_payment_sign_transaction(
                *signer,
                *to,
                nonce,
                *amount,
                *fee,
                Network::Testnet,
            )?;

            let signed_tx_size = signed_tx.encoded_len();
            let response = rpc_client
                .transaction_service()
                .send_transaction(signed_tx)
                .await?;

            json!({
                "tx_size" : signed_tx_size,
                "tx_hash" : H256::from_slice(&response.get_ref().hash),
            })
        }
        ClientCommands::GetTxpool => {
            let txpool_content = rpc_client
                .transaction_service()
                .get_txpool_content(Empty)
                .await?;

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
        ClientCommands::App(a) => handle_app_command(&rpc_client, a).await?,
    };
    Ok(resp)
}
