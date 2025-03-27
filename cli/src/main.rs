mod commands;
mod libs;
mod utils;

use crate::commands::tx::create_deposit_instructions_data;
use clap::{Parser, Subcommand};
use commands::tx::create_transfer_instructions_data;
use utils::{get_current_tree_number, get_deposit_account_metas, get_key_from_file};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Signer, read_keypair_file},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

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
        /// if not provided then assume ATA account
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

        /// receiver viewing public key
        #[arg(short, long)]
        receiver_viewing_public_key: String,

        /// file path to zk proof
        #[arg(short, long)]
        proof_file_path: String,

        /// file path to json file contains all the inputs and outputs
        #[arg(short, long)]
        json_file_path: String,

        /// file path to spending and viewing key
        #[arg(short, long)]
        svk_file_path: String,

        /// file path to keypair
        #[arg(short, long)]
        key_file_path: String,
    },

    /// Withdraw fund to an account
    Withdraw {
        /// token mint account
        /// if not provided then assume native solana
        #[arg(short, long)]
        token_id: Option<String>,

        /// withdraw amount
        #[arg(short, long)]
        amount: u64,

        /// receiver token account
        /// if not provide then assume signer token account
        #[arg(short, long)]
        receiver_token_account: Option<String>,

        /// file path to spending and viewing key
        #[arg(short, long)]
        svk_file_path: String,

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
            let program_id: Pubkey = match Pubkey::from_str(&cli.program_id) {
                Ok(pk) => pk,
                Err(err) => {
                    println!(
                        "{}",
                        format!("Invalid program ID: {}", err.to_string())
                    );

                    return;
                }
            };
             // get user key from file
             let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");

            let token_mint_addr_str = token_id.unwrap_or("So11111111111111111111111111111111111111112".to_string()); // if not provide then assume native sol, use wrapped sol mint account
            let token_mint_addr = match Pubkey::from_str(&token_mint_addr_str) {
                Ok(pk) => pk,
                Err(err) => {
                    println!(
                        "{}",
                        format!("invalid token mint address: {}", err.to_string())
                    );

                    return;
                }
            };  

            // if not provided depositor token address will be 
            // an associated token address
            let depositor_token_addr: Pubkey;
            if depositor_token_address.is_none() {
                let token_mint_addr_str = depositor_token_address.unwrap(); 
                depositor_token_addr = match Pubkey::from_str(&token_mint_addr_str) {
                    Ok(pk) => pk,
                    Err(err) => {
                        println!(
                            "{}",
                            format!("invalid token mint address: {}", err.to_string())
                        );
    
                        return;
                    }
                };                     
            } else {
                depositor_token_addr = get_associated_token_address(&payer.pubkey(), &token_mint_addr);
            }
            
            
            // get system generated spending and viewing key from file
            let (spending_key, viewing_key, deposit_key) = get_key_from_file(svk_file_path).unwrap();

            let serialized_data = match create_deposit_instructions_data(
                &token_mint_addr,
                amount,
                spending_key,
                viewing_key,
                deposit_key,
                memo,
            ) {
                Ok(data) => data,
                Err(err) => {
                    println!(
                        "{}",
                        format!("failed to create instruction data: {}", err.to_string())
                    );

                    return;
                }
            };

            // get current tree number to fetch the correct commitments account info
            let tree_number = match get_current_tree_number(url.clone(), &program_id) {
                Ok(number) => number,
                Err(err) => {
                    println!(
                        "{}",
                        format!("failed to fetch current tree number: {}", err.to_string())
                    );

                    return;
                }
            };

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

            let accounts = get_deposit_account_metas(
                url.clone(),
                &payer.pubkey(),
                &depositor_token_addr,
                &token_mint_addr,
                &program_id,
                tree_number,
            )
            .unwrap();

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
            token_id,
            receiver_viewing_public_key,
            svk_file_path,
            key_file_path,
            json_file_path,
            proof_file_path,
        } => {
            let program_id = match Pubkey::from_str(&cli.program_id) {
                Ok(pk) => pk,
                Err(err) => {
                    println!(
                        "{}",
                        format!("Invalid program ID: {}", err.to_string())
                    );

                    return;
                }
            };
            let token_mint_addr_str = token_id.unwrap_or("So11111111111111111111111111111111111111112".to_string()); // if not provide then assume native sol, use wrapped sol mint account
            let token_mint_addr = match Pubkey::from_str(&token_mint_addr_str) {
                Ok(pk) => pk,
                Err(err) => {
                    println!(
                        "{}",
                        format!("invalid token mint address: {}", err.to_string())
                    );

                    return;
                }
            };            
            
            // get user key from file
            let payer = read_keypair_file(&key_file_path).expect("Failed to load payer keypair");
            // get system generated spending and viewing key from file
            let (spending_key, viewing_key, _deposit_key) = get_key_from_file(svk_file_path).unwrap();

            // let serialized_data = match create_transfer_instructions_data(
            //     &token_mint_addr,
            //     receiver_viewing_public_key.as_bytes().to_vec(),
            //     spending_key,
            //     viewing_key,
            //     deposit_key,
            //     memo,
            //     ,
            //     ,
                
            // ) {
            //     Ok(data) => data,
            //     Err(err) => {
            //         println!(
            //             "{}",
            //             format!("failed to create instruction data: {}", err.to_string())
            //         );

            //         return;
            //     }
            // };

            // // Create instruction
            // let instruction = Instruction {
            //     program_id,
            //     accounts,
            //     data: serialized_data,
            // };

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
            token_id,
            receiver_token_account,
            svk_file_path,
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
