use darksol::{
    CommitmentCipherText, DepositRequest, PreCommitments, ShieldCipherText, TransferRequest,
    WithdrawRequest,
};
use solana_sdk::pubkey::Pubkey;
use types::{generate_nullifier, utxo::UTXO};

use crate::{utils::generate_random_bytes, TransferInput, TransferOutput};

pub fn create_deposit_instructions_data(
    token_id: &Pubkey,
    amount: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    deposit_key: Vec<u8>,
    memo: String,
) -> Result<Vec<u8>, String> {
    let utxo = UTXO::new(
        spending_key.clone(),
        viewing_key.clone(),
        token_id.to_bytes().to_vec(),
        generate_random_bytes(32),
        generate_random_bytes(32),
        amount,
        memo,
    );

    let pre_commitment =
        PreCommitments::new(amount, token_id.to_bytes().to_vec(), utxo.utxo_public_key());
    let deposit_ciphertext = utxo.encrypt_for_deposit(viewing_key.clone(), deposit_key.clone());

    let shield_cipher_text = ShieldCipherText::new(
        deposit_ciphertext.shield_key,
        deposit_ciphertext.cipher,
        utxo.nonce(),
    );

    let request = DepositRequest::new(pre_commitment, shield_cipher_text);
    let instructions_data = match borsh::to_vec(&request) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    Ok(instructions_data)
}

pub fn create_transfer_instructions_data(
    token_id: &Pubkey,
    receiver_public_viewing_key: Vec<u8>,
    proof: Vec<u8>,
    inputs: Vec<TransferInput>,
    outputs: Vec<TransferOutput>,
    merkle_root: Vec<u8>,
    tree_number: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut sum_in: u64 = 0;
    let mut sum_out: u64 = 0;

    for idx in 0..inputs.len() {
        sum_in += inputs[idx].amount;

        let nullifer = generate_nullifier(viewing_key.clone(), inputs[idx].merkle_leaf_index);
        nullifiers.push(nullifer);
    }

    for idx in 0..outputs.len() {
        sum_out += outputs[idx].amount;

        let utxo = UTXO::new(
            spending_key.clone(),
            receiver_public_viewing_key.clone(),
            token_id.to_bytes().to_vec(),
            generate_random_bytes(32),
            generate_random_bytes(32),
            outputs[idx].amount,
            outputs[idx].memo.clone(),
        );

        commiments.push(utxo.utxo_hash().clone());

        let cipher_text = utxo.encrypt(viewing_key.clone());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce().clone(),
            outputs[idx].memo.as_bytes().to_vec().clone(),
        );
        commitment_cipher_texts.push(commitment_cipher_text);
    }

    // check total input and output
    // if input < output then it is insurficent balance and should return an error
    // if input > output then add a new UTXO for sender represent their new balance
    if sum_in < sum_out {
        return Err(format!("total inputs less than total outputs"));
    } else if sum_in > sum_out {
        let remain_balance = sum_in - sum_out;

        let utxo = UTXO::new(
            spending_key.clone(),
            viewing_key.clone(),
            token_id.to_bytes().to_vec(),
            generate_random_bytes(32),
            generate_random_bytes(32),
            remain_balance,
            "".to_string(),
        );

        commiments.push(utxo.utxo_hash().clone());

        let cipher_text = utxo.encrypt(viewing_key.clone());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce(),
            "".to_string().as_bytes().to_vec(),
        );
        commitment_cipher_texts.push(commitment_cipher_text);
    }

    let mut transfer_request =
        TransferRequest::new(proof, merkle_root, tree_number, commitment_cipher_texts);

    nullifiers.iter().for_each(|nullifier| {
        transfer_request.push_nullifiers(nullifier.clone());
    });

    commiments.iter().for_each(|commitment| {
        transfer_request.push_encrypted_commitments(commitment.clone());
    });

    let instructions_data = match borsh::to_vec(&transfer_request) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    Ok(instructions_data)
}

pub fn create_withdraw_instructions_data(
    token_id: &Pubkey,
    proof: Vec<u8>,
    amount: u64,
    inputs: Vec<TransferInput>,
    merkle_root: Vec<u8>,
    tree_number: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
) -> Result<(Vec<u8>, bool), String> {
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut sum_in: u64 = 0;
    let mut insert_new_commitment = false;

    for idx in 0..inputs.len() {
        sum_in += inputs[idx].amount;

        let nullifer = generate_nullifier(viewing_key.clone(), inputs[idx].merkle_leaf_index);
        nullifiers.push(nullifer);
    }

    if sum_in > amount {
        let remain_balance = sum_in - amount;

        let utxo = UTXO::new(
            spending_key.clone(),
            viewing_key.clone(),
            token_id.to_bytes().to_vec(),
            generate_random_bytes(32),
            generate_random_bytes(32),
            remain_balance,
            "".to_string(),
        );

        let commiment = utxo.utxo_hash().clone();
        commiments.push(commiment);

        let cipher_text = utxo.encrypt(viewing_key.clone());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce(),
            "".to_string().as_bytes().to_vec(),
        );

        commitment_cipher_texts.push(commitment_cipher_text);
        insert_new_commitment = true;
    } else if sum_in < amount {
        return Err(format!("insufficient balances"));
    }

    let mut withdraw_request = WithdrawRequest::new(
        proof,
        merkle_root,
        tree_number,
        amount,
        token_id.to_bytes().to_vec(),
        commitment_cipher_texts,
    );

    nullifiers.iter().for_each(|nullifier| {
        withdraw_request.push_nullifiers(nullifier.clone());
    });

    commiments.iter().for_each(|commitment| {
        withdraw_request.push_encrypted_commitment(commitment.clone());
    });


    let instructions_data = match borsh::to_vec(&withdraw_request) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    Ok((instructions_data, insert_new_commitment))
}
