use borsh::BorshDeserialize;
use darksol::{derive_pda, state::CommitmentsManagerAccount};
use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_program::system_program;
use solana_sdk::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::fs;

use crate::libs::{JsonContent, TransferInput, TransferOutput};

pub const CONTENT_LENGTH: usize = 96;

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..length).map(|_| rng.random()).collect()
}

pub fn get_current_tree_number(rpc_url: String, program_id: &Pubkey) -> Result<u64, String> {
    let client = RpcClient::new(rpc_url.to_string());
    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);

    let data = match client.get_account_data(&commitments_manager_pda) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };
    let manager_acc = match CommitmentsManagerAccount::try_from_slice(&data) {
        Ok(value) => value,
        Err(err) => {
            return Err(err.to_string());
        }
    };

    Ok(manager_acc.incremental_tree_number)
}

pub fn get_deposit_account_metas(
    rpc_url: String,
    user_wallet: &Pubkey,
    user_token_addr: &Pubkey,
    token_mint_address: &Pubkey,
    program_id: &Pubkey,
    tree_number: u64,
) -> Result<Vec<AccountMeta>, ProgramError> {
    let client = RpcClient::new(rpc_url.to_string());
    let mut query_addresses: Vec<Pubkey> = vec![];
    let mut account_metas: Vec<AccountMeta> = vec![];

    let (funding_pda, _bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], program_id);
    query_addresses.push(funding_pda);

    query_addresses.push(user_wallet.clone());
    query_addresses.push(user_token_addr.clone());

    let pda_token_addr = get_associated_token_address(&funding_pda, token_mint_address);
    query_addresses.push(pda_token_addr);

    query_addresses.push(token_mint_address.clone());

    let (commitments_pda, _bump_seed) = derive_pda(tree_number, program_id);
    query_addresses.push(commitments_pda);

    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);
    query_addresses.push(commitments_manager_pda);

    query_addresses.push(TOKEN_PROGRAM_ID);
    query_addresses.push(system_program::ID);

    for idx in 0..query_addresses.len() {
        let res = client.get_account(&query_addresses[idx]);

        // only the user wallet is the signer
        let mut is_signer = false;
        if idx == 1 {
            is_signer = true;
        }

        match res {
            Ok(account) => {
                let account_meta = if account.executable {
                    AccountMeta::new(query_addresses[idx], is_signer) // If executable, just readable
                } else {
                    AccountMeta::new_readonly(query_addresses[idx], is_signer) // Non-executable: read-only
                };

                account_metas.push(account_meta);
            }
            Err(err) => {
                println!("❌ Error fetching account info: {}", err);
            }
        }
    }

    Ok(account_metas)
}

pub fn get_transfer_account_metas(
    program_id: &Pubkey,
    rpc_url: String,
    user_wallet: &Pubkey,
    tree_number: u64,
    newest_tree_number: u64,
) -> Result<Vec<AccountMeta>, ProgramError> {
    let client = RpcClient::new(rpc_url.to_string());
    let mut query_addresses: Vec<Pubkey> = vec![];
    let mut account_metas: Vec<AccountMeta> = vec![];

    query_addresses.push(user_wallet.clone());

    let (spent_commitments_pda, _bump_seed) = derive_pda(tree_number, program_id);
    query_addresses.push(spent_commitments_pda);

    let (current_commitments_pda, _bump_seed) = derive_pda(newest_tree_number, program_id);
    query_addresses.push(current_commitments_pda);

    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);
    query_addresses.push(commitments_manager_pda);

    for idx in 0..query_addresses.len() {
        let res = client.get_account(&query_addresses[idx]);

        // only the user wallet is the signer
        let mut is_signer = false;
        if idx == 0 {
            is_signer = true;
        }

        match res {
            Ok(account) => {
                let account_meta = if account.executable {
                    AccountMeta::new(query_addresses[idx], is_signer) // If executable, just readable
                } else {
                    AccountMeta::new_readonly(query_addresses[idx], is_signer) // Non-executable: read-only
                };

                account_metas.push(account_meta);
            }
            Err(err) => {
                println!("❌ Error fetching account info: {}", err);
            }
        }
    }

    Ok(account_metas)
}

pub fn get_withdraw_account_metas(
    program_id: &Pubkey,
    rpc_url: String,
    user_wallet: &Pubkey,
    user_token_account: &Pubkey,
    token_mint_address: &Pubkey,
    tree_number: u64,
    newest_tree_number: u64,
    is_insert_new_commitment: bool,
) -> Result<Vec<AccountMeta>, ProgramError> {
    let client = RpcClient::new(rpc_url.to_string());
    let mut query_addresses: Vec<Pubkey> = vec![];
    let mut account_metas: Vec<AccountMeta> = vec![];

    let (funding_pda, _bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], program_id);
    query_addresses.push(funding_pda);

    let (spent_commitments_pda, _bump_seed) = derive_pda(tree_number, program_id);
    query_addresses.push(spent_commitments_pda);

    query_addresses.push(user_wallet.clone());
    query_addresses.push(user_token_account.clone());

    let funding_token_account = get_associated_token_address(&funding_pda, token_mint_address);
    query_addresses.push(funding_token_account); 

    query_addresses.push(TOKEN_PROGRAM_ID);

    if is_insert_new_commitment {
        let (current_commitments_pda, _bump_seed) = derive_pda(newest_tree_number, program_id);
        query_addresses.push(current_commitments_pda);
    
        let (commitments_manager_pda, _bump_seed) =
            Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);
        query_addresses.push(commitments_manager_pda);
    }

    for idx in 0..query_addresses.len() {
        let res = client.get_account(&query_addresses[idx]);

        // only the user wallet is the signer
        let mut is_signer = false;
        if idx == 0 {
            is_signer = true;
        }

        match res {
            Ok(account) => {
                let account_meta = if account.executable {
                    AccountMeta::new(query_addresses[idx], is_signer) // If executable, just readable
                } else {
                    AccountMeta::new_readonly(query_addresses[idx], is_signer) // Non-executable: read-only
                };

                account_metas.push(account_meta);
            }
            Err(err) => {
                println!("❌ Error fetching account info: {}", err);
            }
        }
    }

    Ok(account_metas)
}

pub fn get_key_from_file(path: String) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    let res = fs::read(path);

    match res {
        Ok(content) => {
            // 32 bytes for each key
            if content.len() != CONTENT_LENGTH {
                return Err(format!(
                    "invalid file content length, should be {} but got {}",
                    CONTENT_LENGTH,
                    content.len()
                ));
            }

            let spending_key = content[..32].to_vec();
            let viewing_key = content[32..64].to_vec();
            let deposit_key = content[64..96].to_vec();

            Ok((spending_key, viewing_key, deposit_key))
        }
        Err(err) => {
            return Err(format!("cannot read from file: {}", err.to_string(),));
        }
    }
}

pub fn read_json_file(
    file_path: String,
) -> Result<(Vec<TransferInput>, Vec<TransferOutput>), String> {
    let res = fs::read(file_path);

    match res {
        Ok(content) => {
            let json_content: JsonContent = match serde_json::from_slice(content.as_slice()) {
                Ok(data) => data,
                Err(err) => {
                    return Err(format!("fail to parse from json: {}", err.to_string(),));
                }
            };

            Ok((json_content.inputs, json_content.outputs))
        }
        Err(err) => {
            return Err(format!("cannot read from file: {}", err.to_string(),));
        }
    }
}

pub fn get_proof_from_file(file_path: String) -> Result<Vec<u8>, String> {
    Ok(vec![])
}
