use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer, read_keypair_file},
    transaction::Transaction,
};
use std::str::FromStr;

/// Solana CLI Example
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// RPC URL (default: Devnet)
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit money into program pool
    Deposit {
        /// Program ID
        #[arg(short, long)]
        program_id: String,

        /// deposit amount
        #[arg(short, long)]
        amount: u64,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },

    /// Transfer money privately
    Transfer {
        /// Program ID
        #[arg(short, long)]
        program_id: String,

        /// transfer amount
        #[arg(short, long)]
        amount: u64,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },

    /// Withdraw fund to an account
    Withdraw {
        /// Program ID
        #[arg(short, long)]
        program_id: String,

        /// withdraw amount
        #[arg(short, long)]
        amount: u64,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let rpc_client = RpcClient::new_with_commitment(cli.rpc_url, CommitmentConfig::confirmed());

    match cli.command {
        Commands::Deposit {
            program_id,
            amount,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&program_id).expect("Invalid program ID");
            let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");

            let message = Message::new(&[], Some(&payer.pubkey()));
            let mut transaction = Transaction::new_unsigned(message);

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            let signature = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();
            println!("✅ Transaction successful! Signature: {}", signature);
        }

        Commands::Transfer {
            program_id,
            amount,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&program_id).expect("Invalid program ID");
            let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");

            let message = Message::new(&[], Some(&payer.pubkey()));
            let mut transaction = Transaction::new_unsigned(message);

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            let signature = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();
            println!("✅ Transaction successful! Signature: {}", signature);
        }

        Commands::Withdraw {
            program_id,
            amount,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&program_id).expect("Invalid program ID");
            let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");

            let message = Message::new(&[], Some(&payer.pubkey()));
            let mut transaction = Transaction::new_unsigned(message);

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            let signature = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();
            println!("✅ Transaction successful! Signature: {}", signature);
        }
    }
}
