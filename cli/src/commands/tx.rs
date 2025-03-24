use darksol::{PreCommitments, DepositRequest, TransferRequest, ShieldCipherText};
use solana_sdk::program_error::ProgramError;
use types::utxo::UTXO;

use crate::libs::generate_random_bytes;

pub fn create_deposit_instructions_data (
    token_id: String,
    amount: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    memo: String,
) -> Result<Vec<u8>, ProgramError>  {
    let utxo= UTXO::new(
        spending_key.clone(),
        viewing_key.clone(),
        token_id.as_bytes().to_vec(),
        generate_random_bytes(32),
        generate_random_bytes(32),
        amount,
        memo,
    );

    let pre_commitment = PreCommitments::new(amount, utxo.utxo_public_key());
    let mut shield_cipher_text = ShieldCipherText::new(
        utxo.master_public_key(),
    );

    // TODO: encrypted cipher text

    shield_cipher_text.push_data(value);
    let request = DepositRequest::new(pre_commitment, shield_cipher_text);
    let instructions_data = borsh::to_vec(&request)?;

    Ok(instructions_data)
}

pub fn create_transfer_instructions_data (
    token_id: String,
    receiver: String,
    amount: u64,
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    memo: String,
) -> Result<Vec<u8>, ProgramError>  {
    let utxo= UTXO::new(
        spending_key.clone(),
        viewing_key.clone(),
        token_id.as_bytes().to_vec(),
        generate_random_bytes(32),
        generate_random_bytes(32),
        amount,
        memo,
    );

    let pre_commitment = PreCommitments::new(amount, utxo.utxo_hash());
    let mut shield_cipher_text = ShieldCipherText::new(
        utxo.master_public_key(),
    );

    // TODO: encrypted cipher text

    shield_cipher_text.push_data(value);
    let request = DepositRequest::new(pre_commitment, shield_cipher_text);
    let instructions_data = borsh::to_vec(&request)?;

    Ok(instructions_data)
}