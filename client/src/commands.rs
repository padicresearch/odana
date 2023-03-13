use crate::rpc::{GetAccountRequest, GetDescriptorRequest};
use crate::util::{parse_cli_args_to_json, RpcMethod};
use crate::Client;
use clap::{Args, Subcommand};
use primitive_types::H256;
use prost::Message;

use serde_json::{json, Value};
use std::collections::HashMap;

use primitive_types::address::Address;
use prost_reflect::{DescriptorPool, DynamicMessage, SerializeOptions};
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use transaction::{make_payment_sign_transaction, make_signed_transaction};
use types::account::{get_address_from_package_name, get_address_from_secret_key};
use types::network::Network;
use types::prelude::Empty;
use types::tx::{ApplicationCall, CreateApplication, TransactionData};

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
    #[clap(long)]
    call: String,
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
    #[clap(long)]
    call: String,
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

pub async fn handle_app_command(
    rpc_client: &Client,
    command: &AppArgsCommands,
) -> anyhow::Result<Value> {
    let resp = match &command.command {
        AppCommands::Call(CallArgs {
            app,
            sign_args,
            call,
            params,
        }) => {
            let signer = sign_args.signer;
            let value = sign_args.value;
            let tip = sign_args.tip;

            let call = RpcMethod::parse(call)?;
            let signer_address = get_address_from_secret_key(signer, Network::Testnet)?;

            let nonce = rpc_client
                .account_service()
                .get_nonce(GetAccountRequest {
                    address: Some(signer_address),
                })
                .await?
                .get_ref()
                .nonce;

            let app_id = get_address_from_package_name(app, Network::Testnet)?;

            //Get App Descriptor bytes
            let mut rt = rpc_client.runtime_api_service();
            let descriptor = rt
                .get_descriptor(GetDescriptorRequest {
                    app_id: Some(app_id),
                })
                .await?
                .into_inner()
                .descriptor;

            let descriptor =
                DescriptorPool::decode(descriptor.as_slice()).expect("failed to descriptor pool");

            let service = descriptor
                .get_service_by_name(call.service_name())
                .expect("service not found in descriptor");
            let method_to_call = service
                .methods()
                .find(|method| method.name() == call.method_name())
                .expect("method not found in descriptor");

            let input = method_to_call.input();
            let json_value = parse_cli_args_to_json(params.iter())?;
            let message = DynamicMessage::deserialize(input, json_value)?;
            let mut serializer = serde_json::Serializer::new(vec![]);
            message.serialize(&mut serializer)?;
            let call_json: Value = serde_json::from_slice(serializer.into_inner().as_slice())?;

            let data = TransactionData::Call(ApplicationCall {
                app_id,
                service: call.service_id(),
                method: call.method_id(),
                args: message.encode_to_vec(),
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
                "tx_hash" :  response.get_ref().hash,
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
                    address: Some(signer_address),
                })
                .await?
                .get_ref()
                .nonce;

            let mut file = File::open(bin_path)?;
            let mut binary = Vec::with_capacity(file.metadata()?.len() as usize);
            let _ = file.read_to_end(&mut binary)?;
            let code_hash = crypto::keccak256(&binary);
            let data = TransactionData::Create(CreateApplication {
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
                "tx_hash" : response.get_ref().hash,
            })
        }
        AppCommands::Query(AppQueryArgs { app, call, params }) => {
            let call = RpcMethod::parse(call)?;
            let app_id = get_address_from_package_name(app, Network::Testnet)?;

            let mut rt = rpc_client.runtime_api_service();
            let descriptor = rt
                .get_descriptor(GetDescriptorRequest {
                    app_id: Some(app_id),
                })
                .await?
                .into_inner()
                .descriptor;

            let descriptor =
                DescriptorPool::decode(descriptor.as_slice()).expect("failed to descriptor pool");

            let service = descriptor
                .get_service_by_name(call.service_name())
                .expect("service not found in descriptor");
            let method_to_call = service
                .methods()
                .find(|method| method.name() == call.method_name())
                .expect("method not found in descriptor");

            let json_value = parse_cli_args_to_json(params.iter())?;
            let message = DynamicMessage::deserialize(method_to_call.input(), json_value)?;
            let mut serializer = serde_json::Serializer::new(vec![]);
            message.serialize(&mut serializer)?;

            let query = ApplicationCall {
                app_id,
                service: call.service_id(),
                method: call.method_id(),
                args: message.encode_to_vec(),
            };

            let response = rpc_client
                .runtime_api_service()
                .query_runtime(query)
                .await?;

            let message = DynamicMessage::decode(
                method_to_call.output(),
                response.get_ref().data.as_slice(),
            )?;

            let mut serializer = serde_json::Serializer::new(vec![]);
            message.serialize_with_options(&mut serializer, &SerializeOptions::default().stringify_primitives(true))?;

            let out: Value = serde_json::from_reader(serializer.into_inner().as_slice())?;
            out
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
                    address: Some(*address),
                })
                .await?;
            Value::Number(balance.get_ref().balance.into())
        }
        ClientCommands::GetNonce(AddressArg { address }) => {
            let nonce = rpc_client
                .account_service()
                .get_nonce(GetAccountRequest {
                    address: Some(*address),
                })
                .await?;
            Value::Number(nonce.get_ref().nonce.into())
        }
        ClientCommands::GetAccountState(AddressArg { address }) => {
            let account_state = rpc_client
                .account_service()
                .get_account_state(GetAccountRequest {
                    address: Some(*address),
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
                    address: Some(signer_address),
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
                "tx_hash" : response.get_ref().hash,
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
                .map(|r| (r.address.unwrap_or_default(), &r.txs))
                .collect();

            let pending_txs: HashMap<_, _> = txpool_content
                .get_ref()
                .pending
                .iter()
                .map(|r| (r.address.unwrap_or_default(), &r.txs))
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
