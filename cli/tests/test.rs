#![cfg(test)]

use std::vec;

use borsh::{BorshDeserialize, BorshSerialize};
use cli::{
    solana::transaction::{
        create_deposit_instructions_data, create_deposit_instructions_data_test,
        create_transfer_instructions_data, create_transfer_instructions_data_test,
        create_withdraw_instructions_data, create_withdraw_instructions_data_test,
    },
    utils::{
        TransferInput, TransferOutput, generate_random_bytes, get_proof_from_file, read_json_file,
    },
};
use darksol::{
    DepositRequest, SP1Groth16Proof, derive_pda, entrypoint::process_instruction,
    utils::account::get_associated_token_address_and_bump_seed,
};
use litesvm::LiteSVM;
use solana_program::system_program::ID as SYSTEM_PROGRAM_ID;
use solana_program_test::{ProgramTest, processor};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
    tools::account,
};
use spl_token::instruction::sync_native;
use types::merkle::MerkleTreeSparse;
use types::{PublicData, UTXO};

#[tokio::test]
async  fn test() {
    let program_id = Pubkey::new_unique();

    let (mut banks_client, payer, recent_blockhash) =
        ProgramTest::new("darksol", program_id, processor!(process_instruction))
            .start()
            .await;

    let depositor_keypair = solana_sdk::signature::Keypair::new();
    let depositor_pubkey = depositor_keypair.pubkey();

    let depositor_deposit_key = solana_sdk::signature::Keypair::new();
    let depositor_view_key = solana_sdk::signature::Keypair::new();
    let depositor_spend_key = solana_sdk::signature::Keypair::new();

    let receiver_keypair = solana_sdk::signature::Keypair::new();
    let receiver_pubkey = receiver_keypair.pubkey();

    let receiver_deposit_key = solana_sdk::signature::Keypair::new();
    let receiver_view_key = solana_sdk::signature::Keypair::new();
    let receiver_spend_key = solana_sdk::signature::Keypair::new();

    // Create and fund the depositor account
    // let depositor_account = solana_sdk::account::Account {
    //     lamports: 1_000_000, // Initial lamports for the depositor
    //     data: vec![],
    //     owner: program_id,
    //     executable: false,
    //     rent_epoch: 0,
    // };

    let data_len = 0;
    let rent_exemption_amount = solana_sdk::rent::Rent::default().minimum_balance(data_len);

    let create_acc_ix = system_instruction::create_account(
        &payer.pubkey(),                        // payer
        &depositor_pubkey,                      // new account
        rent_exemption_amount + 10_000_000_000, // rent exemption fee
        data_len as u64,                        // space reseved for new account
        &SYSTEM_PROGRAM_ID,                     //assigned program address
    );

    let mut transaction = Transaction::new_with_payer(&[create_acc_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer, &depositor_keypair], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let depositor_balance = banks_client.get_balance(depositor_pubkey).await.unwrap();

    println!("Depositor balance: {}", depositor_balance);

    // initialize

    let mut account_metas: Vec<AccountMeta> = vec![];

    account_metas.push(AccountMeta::new(depositor_pubkey, true));

    let (funding_pda, _bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], &program_id);
    account_metas.push(AccountMeta::new(funding_pda, false));

    let (commitments_pda, _bump_seed) = derive_pda(1, &program_id);
    account_metas.push(AccountMeta::new(commitments_pda, false));

    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], &program_id);
    account_metas.push(AccountMeta::new(commitments_manager_pda, false));

    account_metas.push(AccountMeta::new(SYSTEM_PROGRAM_ID, false));

    let instruction = Instruction {
        program_id,
        accounts: account_metas,
        data: vec![3],
    };

    let mut transaction =
        Transaction::new_with_payer(&[instruction], Some(&depositor_keypair.pubkey()));

    transaction.sign(&[&depositor_keypair], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let ata = get_associated_token_address(&depositor_pubkey, &spl_token::native_mint::ID);

    let amount = 1 * 10_u64.pow(9); /* Wrapped SOL's decimals is 9, hence amount to wrap is 1 SOL */

    // create token account for wrapped sol
    let create_ata_ix = create_associated_token_account_idempotent(
        &depositor_pubkey,
        &depositor_pubkey,
        &spl_token::native_mint::ID,
        &spl_token::ID,
    );

    let transfer_ix = system_instruction::transfer(&depositor_pubkey, &ata, amount);
    let sync_native_ix = sync_native(&spl_token::ID, &ata).unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[create_ata_ix, transfer_ix, sync_native_ix],
        Some(&depositor_pubkey),
    );

    transaction.sign(
        &[&depositor_keypair],
        recent_blockhash,
    );

    let res = banks_client.process_transaction(transaction).await;

    match res {
        Ok(_) => println!("Deposit transaction successful"),
        Err(err) => println!("Deposit transaction failed: {:?}", err),
    }

    let funding_balance = banks_client.get_balance(funding_pda).await.unwrap();

    println!("Funding balance: {}", funding_balance);

    // deposit

    let (mut deposit_data, deposit_utxo) = match create_deposit_instructions_data_test(
        &spl_token::native_mint::ID,
        amount,
        depositor_spend_key.pubkey().to_bytes().to_vec(),
        depositor_view_key.pubkey().to_bytes().to_vec(),
        depositor_deposit_key.pubkey().to_bytes().to_vec(),
        "test deposit".to_string(),
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
    let tree_number = 1;

    let ata = get_associated_token_address(&depositor_pubkey, &spl_token::native_mint::ID);

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
    let mut account_metas: Vec<AccountMeta> = vec![];

    let (funding_pda, bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], &program_id);
    account_metas.push(AccountMeta::new(funding_pda, false));

    account_metas.push(AccountMeta::new(depositor_pubkey, true));

    account_metas.push(AccountMeta::new(ata, false));

    let (funding_ata, ata_bump) = get_associated_token_address_and_bump_seed(
        &funding_pda,
        &spl_token::native_mint::ID,
        &spl_associated_token_account::ID,
        &spl_token::ID,
    );

    let (funding_ata, ata_bump) = Pubkey::find_program_address(&[b"funding_ata"], &program_id);

    println!("bump {:?}", ata_bump);
    account_metas.push(AccountMeta::new(funding_ata, false));

    account_metas.push(AccountMeta::new_readonly(spl_token::native_mint::ID, false));

    let (commitments_pda, _bump_seed) = derive_pda(tree_number, &program_id);
    account_metas.push(AccountMeta::new(commitments_pda, false));

    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], &program_id);
    account_metas.push(AccountMeta::new(commitments_manager_pda, false));

    account_metas.push(AccountMeta::new_readonly(spl_token::ID, false));

    account_metas.push(AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false));

    account_metas.push(AccountMeta::new_readonly(
        solana_program::rent::sysvar::ID,
        false,
    ));

    account_metas.push(AccountMeta::new_readonly(
        spl_associated_token_account::ID,
        false,
    ));

    for i in account_metas.iter() {
        println!("Account: {}", i.pubkey);
    }

    // insert variant bytes
    deposit_data.insert(0, 0);
    // Create instruction
    let instruction = Instruction {
        program_id,
        accounts: account_metas,
        data: deposit_data,
    };

    let message = Message::new(&[instruction], Some(&depositor_pubkey));
    let mut transaction = Transaction::new_unsigned(message);

    transaction.sign(&[&depositor_keypair], recent_blockhash);

    let res = banks_client.process_transaction(transaction).await;

    match res {
        Ok(_) => println!("Deposit transaction successful"),
        Err(err) => println!("Deposit transaction failed: {:?}", err),
    }

    // transfer

    let mut tree: MerkleTreeSparse<32> = MerkleTreeSparse::new(1);

    tree.insert(vec![deposit_utxo.utxo_hash()]);

    let inputs = vec![TransferInput {
        amount: amount,
        merkle_leaf_index: 0,
    }];
    let outputs = vec![TransferOutput {
        amount: amount,
        memo: "a".to_string(),
    }];

    let (mut serialized_data, new_commitment) = match create_transfer_instructions_data_test(
        &spl_token::native_mint::ID,
        receiver_view_key.pubkey().to_bytes().to_vec(),
        inputs,
        outputs,
        tree.root(),
        tree_number,
        depositor_spend_key.secret().to_bytes().to_vec(),
        depositor_view_key.secret().to_bytes().to_vec(),
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
    let newest_tree_number = 1;

    // get all necessary account meta
    // user wallet
    // spent commitments account
    // current commitments account
    // commitments manager account

    let mut account_metas: Vec<AccountMeta> = vec![];
    account_metas.push(AccountMeta::new(depositor_pubkey.clone(), true));

    let (spent_commitments_pda, _bump_seed) = derive_pda(tree_number, &program_id);
    account_metas.push(AccountMeta::new(spent_commitments_pda, false));

    let (current_commitments_pda, _bump_seed) = derive_pda(newest_tree_number, &program_id);
    account_metas.push(AccountMeta::new(current_commitments_pda, false));

    let (commitments_manager_pda, _bump_seed) =
        Pubkey::find_program_address(&[b"commitments_manager_pda"], &program_id);
    account_metas.push(AccountMeta::new(commitments_manager_pda, false));

    // insert variant bytes
    serialized_data.insert(0, 1);
    // Create instruction
    let instruction = Instruction {
        program_id,
        accounts: account_metas,
        data: serialized_data,
    };

    let message = Message::new(&[instruction], Some(&&depositor_pubkey));
    let mut transaction = Transaction::new_unsigned(message);

    transaction.sign(&[&depositor_keypair], recent_blockhash);

    let res = banks_client.process_transaction(transaction).await;

    match res {
        Ok(_) => println!("Transfer transaction successful"),
        Err(err) => println!("Transfer transaction failed: {:?}", err),
    }

    // withdraw
    // create receiver token account
    let receiver_token_addr =
        get_associated_token_address(&receiver_pubkey, &spl_token::native_mint::ID);
    // let create_ata_ix = create_associated_token_account_idempotent(
    //     &receiver_pubkey,
    //     &receiver_pubkey,
    //     &spl_token::native_mint::ID,
    //     &spl_token::ID,
    // );
    // let mut transaction = Transaction::new_with_payer(&[create_ata_ix], Some(&receiver_pubkey));
    // transaction.sign(
    //     &[&receiver_keypair],
    //     recent_blockhash,
    // );
    // let res = banks_client.process_transaction(transaction).await;
    // match res {
    //     Ok(_) => println!("Create ata transaction successful"),
    //     Err(err) => println!("Create ata transaction failed: {:?}", err),
    // }

    let inputs = vec![TransferInput {
        amount,
        merkle_leaf_index: 1,
    }];

    tree.insert(new_commitment);

    let (mut serialized_data, insert_new_commitment) = match create_withdraw_instructions_data_test(
        &spl_token::native_mint::ID,
        amount,
        inputs,
        tree.root(),
        tree_number,
        receiver_spend_key.secret().as_bytes().to_vec(),
        receiver_view_key.secret().as_bytes().to_vec(),
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

    let mut account_metas: Vec<AccountMeta> = vec![];

    let (funding_pda, _bump_seed) = Pubkey::find_program_address(&[b"funding_pda"], &program_id);
    account_metas.push(AccountMeta::new(funding_pda, false));

    let (spent_commitments_pda, _bump_seed) = derive_pda(tree_number, &program_id);
    account_metas.push(AccountMeta::new(spent_commitments_pda, false));

    account_metas.push(AccountMeta::new(receiver_pubkey, false));
    account_metas.push(AccountMeta::new(receiver_token_addr, false));

    // let (funding_ata, ata_bump) = get_associated_token_address_and_bump_seed(
    //     &funding_pda,
    //     &spl_token::native_mint::ID,
    //     &spl_associated_token_account::ID,
    //     &spl_token::ID,
    // );

    let (funding_ata, ata_bump) = Pubkey::find_program_address(&[b"funding_ata"], &program_id);
    account_metas.push(AccountMeta::new(funding_ata, false));

    account_metas.push(AccountMeta::new_readonly(spl_token::ID, false));

    let (current_commitments_pda, _bump_seed) = derive_pda(newest_tree_number, &program_id);
    account_metas.push(AccountMeta::new_readonly(current_commitments_pda, false));

    // let accounts = ctx
    //     .client
    //     .get_withdraw_account_metas(
    //         &program_id,
    //         &ctx.key.key().pubkey(),
    //         &receiver_token_addr,
    //         &token_mint_addr,
    //         tree_number,
    //         newest_tree_number,
    //         insert_new_commitment,
    //     )
    //     .await
    //     .unwrap();

    // insert variant bytes
    serialized_data.insert(0, 2);
    // Create instruction
    let instruction = Instruction {
        program_id,
        accounts: account_metas,
        data: serialized_data,
    };

    let message = Message::new(&[instruction], Some(&&receiver_pubkey));
    let mut transaction = Transaction::new_unsigned(message);

    transaction.sign(&[&receiver_keypair], recent_blockhash);

    let res = banks_client.process_transaction(transaction).await;

    match res {
        Ok(_) => println!("Transfer transaction successful"),
        Err(err) => println!("Transfer transaction failed: {:?}", err),
    }
}
