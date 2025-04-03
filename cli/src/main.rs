use clap::{Parser, Subcommand, command};
use cli::{
    cli::CliContext,
    commands::{
        indexer::IndexerCommands,
        key::{KeyCommands, KeyConfig},
        proof::ProofCommands,
        tx::TxCommands,
    },
    config::CliConfig,
    key::{
        KeyStorage, KeyStorageType,
        raw::{RawKeyStorage, StoredKeypair},
    },
    solana::SolanaClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
#[command(name = "veil-cli")]
#[command(about = "A simple CLI to interact with Veil protocol")]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// RPC URL (default: Devnet)
    #[arg(short, long)]
    rpc_url: Option<String>,

    /// program id
    #[arg(short, long)]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Tx {
        #[command(subcommand)]
        command: TxCommands,
        // // token mint account
        // // if not provided then assume native solana
        // #[arg(short, long)]
        // token_id: Option<String>,
        // // file path to keypair
        // #[arg(short, long)]
        // key_path: String,
    },

    Key {
        #[command(subcommand)]
        command: KeyCommands,

        #[arg(short, long)]
        storage: Option<KeyStorageType>,
    },

    Proof {
        #[command(subcommand)]
        command: ProofCommands,
    },

    Indexer {
        #[command(subcommand)]
        command: IndexerCommands,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = CliConfig::load_or_create(cli.config).unwrap();

    let solana_client = SolanaClient {
        client: RpcClient::new_with_commitment(
            cli.rpc_url.unwrap_or(config.rpc_url),
            CommitmentConfig::confirmed(),
        ),
        ws_client: None,
    };

    let key_storage = match config.key_storage {
        KeyStorageType::Raw | _ => RawKeyStorage::new(config.key_path.clone().into()),
        // KeyStorageType::Encrypted => unimplemented!(),
    };

    let key = key_storage.load_keypair(&config.key).unwrap_or_else(|_| {
        key_storage
            .save_keypair(&config.key, &StoredKeypair::new())
            .unwrap();
        key_storage.load_keypair(&config.key).unwrap()
    });

    let ctx = CliContext {
        client: solana_client,
        program_id: cli.program_id,
        key,
    };

    match cli.command {
        Commands::Key { command, storage } => {
            let key_config = KeyConfig::new(
                PathBuf::from(config.key_path),
                storage.unwrap_or(config.key_storage),
                config.key,
            );
            KeyCommands::handle_command(command, key_config).unwrap()
        }
        Commands::Proof { command } => {
            ProofCommands::handle_command(command);
        }
        Commands::Indexer { command } => {
            IndexerCommands::hanđle_command(command);
        }
        Commands::Tx { command } => {
            TxCommands::handle_command(command, &ctx).await;
        }
    }
}
