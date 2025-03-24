mod commands;
mod libs;

use crate::commands::tx::create_deposit_instructions_data;
use clap::{Parser, Subcommand};
use libs::{get_current_tree_number, get_key_from_file, get_deposit_account_metas};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::{AccountMeta, Instruction}, message::Message, pubkey::Pubkey, signature::{read_keypair_file, Signer}, signer::keypair, transaction::Transaction
};
use std::str::FromStr;
use spl_associated_token_account::get_associated_token_address;

#[derive(Parser)]
#[command(author, version, about)]
#[command(name = "veil-cli")]
#[command(about = "A simple CLI to interact with Veil protocol")]
struct Cli {
    /// RPC URL (default: Devnet)
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// program id
    #[arg(short, long)]
    program_id: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit money into program pool
    Deposit {
        /// token mint account
        /// if not provided then assume native solana
        #[arg(short, long)]
        token_id: Option<String>,

        /// depositor token address
        /// if not provided then assume native solana
        #[arg(short, long)]
        depositor_token_address: Option<String>,

        /// deposit amount
        #[arg(short, long)]
        amount: u64,

        /// memo
        #[arg(short, long)]
        memo: String,

        /// file path to spending and viewing key
        #[arg(short, long)]
        svk_file_path: String,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },

    /// Transfer money privately
    Transfer {
        /// token mint account
        /// if not provided then assume native solana
        #[arg(short, long)]
        token_id: Option<String>,

        /// depositor token address
        /// if not provided then assume native solana
        /// if token id is in the input then we assume
        /// using ATA account
        #[arg(short, long)]
        depositor_token_address: Option<String>,

        /// transfer amount
        #[arg(short, long)]
        amount: u64,

        /// recipient
        #[arg(short, long)]
        receiver: String,

        /// memo
        #[arg(short, long)]
        memo: String,

        /// file path to spending and viewing key
        #[arg(short, long)]
        svk_file_path: String,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },

    /// Withdraw fund to an account
    Withdraw {
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
    let url = &cli.rpc_url;
    let rpc_client = RpcClient::new_with_commitment(url.clone(), CommitmentConfig::confirmed());

    match cli.command {
        Commands::Deposit {
            depositor_token_address,
            token_id,
            amount,
            memo,
            svk_file_path,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&cli.program_id).expect("Invalid program ID");
            // get user key from file
            let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");
            // get system generated spending and viewing key from file
            let (spending_key, viewing_key) = get_key_from_file(svk_file_path).unwrap();

            let result =
                create_deposit_instructions_data(token_id.clone(), amount, spending_key, viewing_key, memo);
            let mut serialized_data: Vec<u8> = Vec::new();

            match result {
                Ok(data) => {
                    serialized_data = data.clone();
                }
                Err(err) => {
                    println!(
                        "{}",
                        format!("failed to create instruction data: {}", err.to_string())
                    )
                }
            }

            // get current tree number to fetch the correct commitments account info
            let mut tree_number: u64 = 0;
            match get_current_tree_number(url.clone(), &program_id) {
                Ok(number) => {
                    tree_number = number;
                }
                Err(err) => {
                    println!(
                        "{}",
                        format!("failed to fetch current tree number: {}", err.to_string())
                    )
                }
            }

            // get all necessary account meta
            // funding_account
            // user_wallet
            // user_token_account
            // pda_token_account
            // mint_account
            // commitments_account
            // commitments_manager_account
            // token_program
            // system_program

            let mut token_mint_addr: Pubkey = Pubkey::new_unique();
            match Pubkey::from_str(&token_id.clone()) {
                Ok(pk) => {
                    token_mint_addr = pk;
                }
                Err(err) => {
                    println!(
                        "{}",
                        format!("invalid token mint address: {}", err.to_string())
                    )
                }
            }

            let accounts = get_deposit_account_metas(url.clone(),  &payer.pubkey(), &token_mint_addr, &program_id, tree_number).unwrap();

            // Create instruction
            let instruction = Instruction {
                program_id,
                accounts,
                data: serialized_data,
            };

            let message = Message::new(&[instruction], Some(&payer.pubkey()));
            let mut transaction = Transaction::new_unsigned(message);

            let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
            transaction.sign(&[&payer], recent_blockhash);

            let signature = rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();
            println!("✅ Transaction successful! Signature: {}", signature);
        }

        Commands::Transfer {
            depositor_token_address,
            token_id,
            receiver,
            amount,
            memo,
            svk_file_path,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&cli.program_id).expect("Invalid program ID");
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
            amount,
            key_file_path,
        } => {
            let program_id = Pubkey::from_str(&cli.program_id).expect("Invalid program ID");
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
