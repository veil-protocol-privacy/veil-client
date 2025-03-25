use darksol::{CommitmentCipherText, DepositRequest, PreCommitments, ShieldCipherText, TransferRequest};
use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};
use types::utxo::UTXO;

use crate::libs::generate_random_bytes;

pub fn create_deposit_instructions_data(
    token_id: &Pubkey,
    amount: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    deposit_key: Vec<u8>,
    memo: String,
) -> Result<Vec<u8>, ProgramError> {
    let utxo = UTXO::new(
        spending_key.clone(),
        viewing_key.clone(),
        token_id.to_bytes().to_vec(),
        generate_random_bytes(32),
        generate_random_bytes(32),
        amount,
        memo,
    );

    let pre_commitment = PreCommitments::new(amount, token_id.to_string(), utxo.utxo_public_key());
    let deposit_ciphertext = utxo.encrypt_for_deposit(viewing_key.clone(), deposit_key.clone());

    let shield_cipher_text = ShieldCipherText::new(deposit_ciphertext.shield_key, deposit_ciphertext.cipher);

    let request = DepositRequest::new(pre_commitment, shield_cipher_text);
    let instructions_data = borsh::to_vec(&request)?;

    Ok(instructions_data)
}

// pub fn create_transfer_instructions_data(
//     token_id: String,
//     receiver: String,
//     proof: Vec<u8>,
//     amount: u64,
//     tree_number: u64,
//     spending_key: Vec<u8>,
//     viewing_key: Vec<u8>,
//     memo: String,
// ) -> Result<Vec<u8>, ProgramError> {
//     let utxo = UTXO::new(
//         spending_key.clone(),
//         viewing_key.clone(),
//         token_id.as_bytes().to_vec(),
//         generate_random_bytes(32),
//         generate_random_bytes(32),
//         amount,
//         memo,
//     );

//     let encypted_data = utxo.encrypt(viewing_key);
//     let commitment_cipher_text = CommitmentCipherText::new(
//         encypted_data.blinded_sender_pubkey, encypted_data.blinded_receiver_pubkey, memo
//     );

//     commitment_cipher_text.push_data(value);

//     let request = TransferRequest::new(proof, vec![], tree_number, );
//     let instructions_data = borsh::to_vec(&request)?;

//     Ok(instructions_data)
// }
