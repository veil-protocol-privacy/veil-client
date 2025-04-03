use super::SolanaClient;
use borsh::BorshDeserialize;
use darksol::{derive_pda, state::CommitmentsManagerAccount};
use solana_sdk::{
    instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey, system_program,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TOKEN_PROGRAM_ID;

impl SolanaClient {
    pub async fn get_current_tree_number(&self, program_id: &Pubkey) -> Result<u64, String> {
        let (commitments_manager_pda, _bump_seed) =
            Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);

        let data = match self.client.get_account_data(&commitments_manager_pda).await {
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

    pub async fn get_deposit_account_metas(
        &self,
        user_wallet: &Pubkey,
        user_token_addr: &Pubkey,
        token_mint_address: &Pubkey,
        program_id: &Pubkey,
        tree_number: u64,
    ) -> Result<Vec<AccountMeta>, ProgramError> {
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
            let res = self.client.get_account(&query_addresses[idx]).await;

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

    pub async fn get_transfer_account_metas(
        &self,
        program_id: &Pubkey,
        user_wallet: &Pubkey,
        tree_number: u64,
        newest_tree_number: u64,
    ) -> Result<Vec<AccountMeta>, ProgramError> {
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
            // only the user wallet is the signer
            let mut is_signer = false;
            if idx == 0 {
                is_signer = true;
            }

            match self.client.get_account(&query_addresses[idx]).await {
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

    pub async fn get_withdraw_account_metas(
        &self,
        program_id: &Pubkey,
        user_wallet: &Pubkey,
        user_token_account: &Pubkey,
        token_mint_address: &Pubkey,
        tree_number: u64,
        newest_tree_number: u64,
        is_insert_new_commitment: bool,
    ) -> Result<Vec<AccountMeta>, ProgramError> {
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
            // only the user wallet is the signer
            let mut is_signer = false;
            if idx == 0 {
                is_signer = true;
            }

            match self.client.get_account(&query_addresses[idx]).await {
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

    pub async fn get_initialize_account_metas(
        &self,
        program_id: &Pubkey,
        payer: Pubkey,
    ) -> Result<Vec<AccountMeta>, ProgramError> {
        let mut account_metas: Vec<AccountMeta> = vec![];

        account_metas.push(AccountMeta::new(payer, true));

        let (funding_pda, _bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], program_id);
        account_metas.push(AccountMeta::new(funding_pda, false));

        let (commitments_pda, _bump_seed) = derive_pda(1, program_id);
        account_metas.push(AccountMeta::new(commitments_pda, false));

        let (commitments_manager_pda, _bump_seed) =
            Pubkey::find_program_address(&[b"commitments_manager_pda"], program_id);
        account_metas.push(AccountMeta::new(commitments_manager_pda, false));

        account_metas.push(AccountMeta::new(system_program::ID, false));

        Ok(account_metas)
    }
}
