use std::str::FromStr;

use base64::{Engine as _, engine::general_purpose};
use clap::Subcommand;
use darksol::derive_pda;
use solana_sdk::{
    instruction::Instruction, message::Message, pubkey::Pubkey, signer::Signer, system_instruction,
    system_program, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{
    cli::CliContext,
    solana::transaction::{
        create_deposit_instructions_data, create_transfer_instructions_data,
        create_withdraw_instructions_data,
    },
    utils::{get_proof_from_file, read_json_file},
};

#[derive(Clone, Subcommand)]
pub enum TxCommands {
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

        // merkle root of the user tree
        #[arg(short, long)]
        merkle_root: String,

        /// tree number
        #[arg(short, long)]
        tree_number: u64,

        /// file path to zk proof
        #[arg(short, long)]
        proof_file_path: String,

        /// file path to json file contains all the inputs and outputs
        #[arg(short, long)]
        json_file_path: String,
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

        // merkle root of the user tree
        #[arg(short, long)]
        merkle_root: String,

        /// tree number
        #[arg(short, long)]
        tree_number: u64,

        /// file path to zk proof
        #[arg(short, long)]
        proof_file_path: String,

        /// file path to json file contains all the inputs and outputs
        #[arg(short, long)]
        json_file_path: String,
    },

    /// Initialize fund to an account
    Initialize {},
}

impl TxCommands {
    pub async fn handle_command(command: TxCommands, ctx: &CliContext) {
        match command {
            TxCommands::Deposit {
                depositor_token_address,
                token_id,
                amount,
                memo,
            } => {
                let program_id: Pubkey = match Pubkey::from_str(&ctx.program_id) {
                    Ok(pk) => pk,
                    Err(err) => {
                        println!("{}", format!("Invalid program ID: {}", err.to_string()));

                        return;
                    }
                };

                let token_mint_addr_str =
                    token_id.unwrap_or("So11111111111111111111111111111111111111112".to_string()); // if not provide then assume native sol, use wrapped sol mint account
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
                    depositor_token_addr = get_associated_token_address(
                        &ctx.key.deposit_key().pubkey(),
                        &token_mint_addr,
                    );
                }

                let mut serialized_data = match create_deposit_instructions_data(
                    &token_mint_addr,
                    amount,
                    ctx.key.spend_key.clone(),
                    ctx.key.view_key.clone(),
                    ctx.key.deposit_key.clone(),
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
                let tree_number = match ctx.client.get_current_tree_number(&program_id).await {
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

                let accounts = ctx
                    .client
                    .get_deposit_account_metas(
                        &ctx.key.key().pubkey(),
                        &depositor_token_addr,
                        &token_mint_addr,
                        &program_id,
                        tree_number,
                    )
                    .await
                    .unwrap();

                // insert variant bytes
                serialized_data.insert(0, 0);
                // Create instruction
                let instruction = Instruction {
                    program_id,
                    accounts,
                    data: serialized_data,
                };

                let message = Message::new(&[instruction], Some(&ctx.key.key().pubkey()));
                let mut transaction = Transaction::new_unsigned(message);

                let recent_blockhash = ctx.client.client.get_latest_blockhash().await.unwrap();
                transaction.sign(&[&ctx.key.key()], recent_blockhash);

                let signature = ctx
                    .client
                    .client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .unwrap();
                println!("✅ Transaction successful! Signature: {}", signature);
            }
            TxCommands::Transfer {
                token_id,
                receiver_viewing_public_key,
                json_file_path,
                proof_file_path,
                tree_number,
                merkle_root,
            } => {
                let program_id = match Pubkey::from_str(&ctx.program_id) {
                    Ok(pk) => pk,
                    Err(err) => {
                        println!("{}", format!("Invalid program ID: {}", err.to_string()));

                        return;
                    }
                };
                let token_mint_addr_str =
                    token_id.unwrap_or("So11111111111111111111111111111111111111112".to_string()); // if not provide then assume native sol, use wrapped sol mint account
                let token_mint_addr = match Pubkey::from_str(&token_mint_addr_str) {
                    Ok(pk) => pk,
                    Err(err) => {
                        return println!(
                            "{}",
                            format!("invalid token mint address: {}", err.to_string())
                        );
                    }
                };

                let (inputs, outputs) = read_json_file(json_file_path).unwrap();
                let proof = get_proof_from_file(proof_file_path).unwrap();

                // decode merkle root string
                let decode = match general_purpose::STANDARD.decode(merkle_root) {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                };

                let mut serialized_data = match create_transfer_instructions_data(
                    &token_mint_addr,
                    receiver_viewing_public_key.as_bytes().to_vec(),
                    proof,
                    inputs,
                    outputs,
                    decode,
                    tree_number,
                    ctx.key.spend_key.clone(),
                    ctx.key.view_key.clone(),
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
                let newest_tree_number = match ctx.client.get_current_tree_number(&program_id).await
                {
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
                // user wallet
                // spent commitments account
                // current commitments account
                // commitments manager account

                let accounts = ctx
                    .client
                    .get_transfer_account_metas(
                        &program_id,
                        &ctx.key.key().pubkey(),
                        tree_number,
                        newest_tree_number,
                    )
                    .await
                    .unwrap();

                // insert variant bytes
                serialized_data.insert(0, 1);
                // Create instruction
                let instruction = Instruction {
                    program_id,
                    accounts,
                    data: serialized_data,
                };

                let message = Message::new(&[instruction], Some(&ctx.key.key().pubkey()));
                let mut transaction = Transaction::new_unsigned(message);

                let recent_blockhash = ctx.client.client.get_latest_blockhash().await.unwrap();
                transaction.sign(&[&ctx.key.key()], recent_blockhash);

                let signature = ctx
                    .client
                    .client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .unwrap();
                println!("✅ Transaction successful! Signature: {}", signature);
            }
            TxCommands::Withdraw {
                amount,
                token_id,
                receiver_token_account,
                tree_number,
                proof_file_path,
                json_file_path,
                merkle_root,
            } => {
                let program_id = match Pubkey::from_str(&ctx.program_id) {
                    Ok(pk) => pk,
                    Err(err) => {
                        println!("{}", format!("Invalid program ID: {}", err.to_string()));

                        return;
                    }
                };
                let token_mint_addr_str =
                    token_id.unwrap_or("So11111111111111111111111111111111111111112".to_string()); // if not provide then assume native sol, use wrapped sol mint account
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

                let (inputs, _outputs) = read_json_file(json_file_path).unwrap();
                let proof = get_proof_from_file(proof_file_path).unwrap();

                // decode merkle root string
                let decode = match general_purpose::STANDARD.decode(merkle_root) {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                };

                let (mut serialized_data, insert_new_commitment) =
                    match create_withdraw_instructions_data(
                        &token_mint_addr,
                        proof,
                        amount,
                        inputs,
                        decode,
                        tree_number,
                        ctx.key.spend_key.clone(),
                        ctx.key.view_key.clone(),
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
                let newest_tree_number = match ctx.client.get_current_tree_number(&program_id).await
                {
                    Ok(number) => number,
                    Err(err) => {
                        println!(
                            "{}",
                            format!("failed to fetch current tree number: {}", err.to_string())
                        );

                        return;
                    }
                };

                // if not provided depositor token address will be
                // an associated token address
                let receiver_token_addr: Pubkey;
                if receiver_token_account.is_none() {
                    let token_mint_addr_str = receiver_token_account.unwrap();
                    receiver_token_addr = match Pubkey::from_str(&token_mint_addr_str) {
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
                    receiver_token_addr =
                        get_associated_token_address(&ctx.key.key().pubkey(), &token_mint_addr);
                }

                // get all necessary account meta
                // funding account
                // spent commitments account
                // user wallet
                // user token account
                // funding token account
                // token program
                //
                // current commitment account
                // commitments manager account

                let accounts = ctx
                    .client
                    .get_withdraw_account_metas(
                        &program_id,
                        &ctx.key.key().pubkey(),
                        &receiver_token_addr,
                        &token_mint_addr,
                        tree_number,
                        newest_tree_number,
                        insert_new_commitment,
                    )
                    .await
                    .unwrap();

                // insert variant bytes
                serialized_data.insert(0, 2);
                // Create instruction
                let instruction = Instruction {
                    program_id,
                    accounts,
                    data: serialized_data,
                };

                let message = Message::new(&[instruction], Some(&ctx.key.key().pubkey()));
                let mut transaction = Transaction::new_unsigned(message);

                let recent_blockhash = ctx.client.client.get_latest_blockhash().await.unwrap();
                transaction.sign(&[&ctx.key.key()], recent_blockhash);

                let signature = ctx
                    .client
                    .client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .unwrap();
                println!("✅ Transaction successful! Signature: {}", signature);
            }
            TxCommands::Initialize {} => {
                let program_id = match Pubkey::from_str(&ctx.program_id) {
                    Ok(pk) => pk,
                    Err(err) => {
                        println!("{}", format!("Invalid program ID: {}", err.to_string()));

                        return;
                    }
                };

                // get all necessary account meta
                // funding account
                // commitment account
                // commitments manager account
                // system program

                let accounts = ctx
                    .client
                    .get_initialize_account_metas(&program_id)
                    .await
                    .unwrap();

                // Create instruction
                let instruction = Instruction {
                    program_id,
                    accounts,
                    data: vec![3],
                };

                let message = Message::new(&[instruction], Some(&ctx.key.key().pubkey()));
                let mut transaction = Transaction::new_unsigned(message);

                let recent_blockhash = ctx.client.client.get_latest_blockhash().await.unwrap();
                transaction.sign(&[&ctx.key.key()], recent_blockhash);

                let signature = ctx
                    .client
                    .client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .unwrap();
                println!("✅ Transaction successful! Signature: {}", signature);
            }
        }
    }
}
