use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use clap::{ArgEnum, Args, Parser, Subcommand};
use client::commands::{handle_client_command, ClientArgsCommands};
use directories::UserDirs;
use indicatif::{ProgressBar, ProgressStyle};

use p2p::identity::NodeIdentity;
use primitive_types::Address;
use tracing::Level;
use types::config::EnvironmentConfig;
use types::network::Network;

pub mod environment;
mod error;
mod node;
pub mod sync;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ArgEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run(RunArgs),
    Identity(IdentityArgs),
    Config(ConfigArgs),
    Account(AccountArgs),
    Client(ClientArgsCommands),
}

#[derive(Args, Debug)]
struct RunArgs {
    #[clap(long)]
    rpc_host: Option<String>,
    #[clap(long)]
    p2p_host: Option<String>,
    #[clap(short, long, value_parser = parse_multaddr)]
    peer: Vec<String>,
    #[clap(short, long)]
    datadir: Option<PathBuf>,
    #[clap(short, long)]
    config_file: Option<PathBuf>,
    #[clap(short, long)]
    identity_file: Option<PathBuf>,
    #[clap(arg_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,
    #[clap(long)]
    expected_pow: Option<f64>,
    #[clap(long, value_parser = parse_miner_address)]
    miner: Option<Address>,
    #[clap(arg_enum, long)]
    network: Option<Network>,
    #[clap(long)]
    p2p_port: Option<u16>,
    #[clap(long)]
    rpc_port: Option<u16>,
}

#[derive(Args, Debug)]
struct IdentityArgs {
    #[clap(subcommand)]
    command: IdentityCommands,
}

#[derive(Subcommand, Debug)]
enum IdentityCommands {
    Generate(IdentityGenerateArgs),
}

#[derive(Args, Debug)]
struct IdentityGenerateArgs {
    #[clap(default_value_t = 26.0)]
    difficulty: f64,
    #[clap(short, long)]
    datadir: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ConfigArgs {
    #[clap(subcommand)]
    command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    Init(SetConfigArgs),
    Update(SetConfigArgs),
    Show,
}

#[derive(Args, Debug)]
struct SetConfigArgs {
    #[clap(long)]
    rpc_host: Option<String>,
    #[clap(long)]
    p2p_host: Option<String>,
    #[clap(long, value_parser = parse_miner_address)]
    miner: Option<Address>,
    #[clap(long)]
    datadir: Option<PathBuf>,
    #[clap(long)]
    identity_file: Option<PathBuf>,
    #[clap(arg_enum, long)]
    log_level: Option<LogLevel>,
    #[clap(long, value_parser = parse_multaddr)]
    peer: Vec<String>,
    #[clap(long)]
    expected_pow: Option<f64>,
    #[clap(long)]
    p2p_port: Option<u16>,
    #[clap(long)]
    rpc_port: Option<u16>,
    #[clap(arg_enum, long)]
    network: Option<Network>,
}

#[derive(Args, Debug)]
struct AccountArgs {
    #[clap(subcommand)]
    command: AccountCommands,
}

#[derive(Subcommand, Debug)]
enum AccountCommands {
    Create(CreateAccountCommandArgs),
}

#[derive(Args, Debug)]
struct CreateAccountCommandArgs {
    #[clap(arg_enum, long)]
    network: Network,
}

fn main() -> Result<()> {
    let args: Cli = Cli::parse();
    match &args.command {
        Commands::Run(args) => {
            node::run(args)?;
        }
        Commands::Identity(args) => match &args.command {
            IdentityCommands::Generate(args) => {
                generate_identity_file(args)?;
            }
        },
        Commands::Config(args) => {
            handle_config_commands(&args.command)?;
        }
        Commands::Account(args) => match &args.command {
            AccountCommands::Create(args) => {
                let account = account::create_account(args.network);
                println!("{}", serde_json::to_string_pretty(&account)?);
            }
        },
        Commands::Client(args) => {
            let rt = tokio::runtime::Runtime::new()?;
            let resp = rt.block_on(async { handle_client_command(args).await })?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
    }

    Ok(())
}

fn create_file_path(datadir: Option<PathBuf>, filename: &str) -> Result<PathBuf> {
    let user_dirs = UserDirs::new().ok_or_else(|| anyhow::anyhow!("user dir not found"))?;
    let path = datadir.unwrap_or_else(|| PathBuf::from(user_dirs.home_dir()).join(".uchain"));
    fs_extra::dir::create_all(path.as_path(), false)?;
    Ok(path.join(filename))
}

fn generate_identity_file(args: &IdentityGenerateArgs) -> Result<()> {
    let identity_file_path = create_file_path(args.datadir.clone(), "identity.json")?;
    let identity_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(identity_file_path.as_path())?;
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")?.tick_strings(&[
            "▫▫▫▫",
            "▪▫▫▫",
            "▫▪▫▫",
            "▫▫▪▫",
            "▫▫▫▪",
            "▪▪▪▪",
        ]),
    );
    pb.set_message(format!(
        "Generating node identity... difficulty({})",
        args.difficulty
    ));
    let identity = NodeIdentity::generate(crypto::make_target(args.difficulty));
    serde_json::to_writer(&identity_file, &identity.export_as_config())?;
    identity_file.sync_all()?;
    pb.finish_with_message(format!(
        "Created identity path: {:?}  difficulty({})",
        identity_file_path, args.difficulty
    ));
    Ok(())
}

fn handle_config_commands(args: &ConfigCommands) -> Result<()> {
    match args {
        ConfigCommands::Init(args) => {
            let mut config = EnvironmentConfig::default();
            if let Some(network) = args.network {
                config.network = network;
            }

            config.peers = args.peer.clone();

            if let Some(network) = args.network {
                config.network = network;
            }

            if let Some(coinbase) = args.miner {
                config.miner = Some(coinbase)
            }

            if let Some(expected_pow) = args.expected_pow {
                config.expected_pow = expected_pow
            }

            if let Some(p2p_host) = &args.p2p_host {
                config.p2p_host = p2p_host.clone()
            }

            if let Some(rpc_host) = &args.rpc_host {
                config.rpc_host = rpc_host.clone()
            }

            if let Some(p2p_port) = args.p2p_port {
                config.p2p_port = p2p_port
            }

            if let Some(rpc_port) = args.rpc_port {
                config.rpc_port = rpc_port
            }

            if let Some(identity_file) = &args.identity_file {
                config.identity_file = Some(identity_file.clone())
            }

            let config_file_path = create_file_path(args.datadir.clone(), "config.json")?;
            let config_file = OpenOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .open(config_file_path.as_path())?;
            serde_json::to_writer(&config_file, &config)?;
            config_file.sync_all()?;

            println!("Created {:?}", config_file_path);
        }
        ConfigCommands::Update(args) => {
            let config_file_path = create_file_path(args.datadir.clone(), "config.json")?;
            let config = sanitize_config_args(args, &config_file_path)?;

            // TODO; Make update safer by using temp file renaming
            let config_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(config_file_path.as_path())?;
            serde_json::to_writer(&config_file, &config)?;
            config_file.sync_all()?;
            println!("Updated {:?}", config_file_path);
        }
        ConfigCommands::Show => {
            let config_file_path = create_file_path(None, "config.json")?;
            let config_file = OpenOptions::new()
                .read(true)
                .open(config_file_path.as_path())?;
            let config: EnvironmentConfig = serde_json::from_reader(&config_file)?;
            let json_string = serde_json::to_string_pretty(&config)?;
            println!("{}", json_string)
        }
    }
    Ok(())
}

fn sanitize_config_args(
    args: &SetConfigArgs,
    config_file_path: &Path,
) -> Result<EnvironmentConfig> {
    let config_file = OpenOptions::new().read(true).open(config_file_path)?;
    let mut config: EnvironmentConfig = serde_json::from_reader(config_file)?;

    if let Some(network) = args.network {
        config.network = network;
    }

    let mut peers: BTreeSet<_> = config.peers.iter().collect();
    peers.extend(args.peer.iter());

    config.peers = peers.into_iter().cloned().collect();

    if let Some(network) = args.network {
        config.network = network;
    }

    if let Some(coinbase) = args.miner {
        config.miner = Some(coinbase)
    }

    if let Some(expected_pow) = args.expected_pow {
        config.expected_pow = expected_pow
    }

    if let Some(p2p_host) = &args.p2p_host {
        config.p2p_host = p2p_host.clone()
    }

    if let Some(rpc_host) = &args.rpc_host {
        config.rpc_host = rpc_host.clone()
    }

    if let Some(p2p_port) = args.p2p_port {
        config.p2p_port = p2p_port
    }

    if let Some(rpc_port) = args.rpc_port {
        config.rpc_port = rpc_port
    }
    Ok(config)
}

pub(crate) fn parse_multaddr(s: &str) -> Result<String, String> {
    match p2p::util::validate_multiaddr(s) {
        Ok(_) => Ok(s.to_string()),
        Err(error) => Err(format!("{}", error)),
    }
}

pub(crate) fn parse_miner_address(s: &str) -> Result<Address, String> {
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
