use borsh::BorshDeserialize;
use darksol::{DepositEvent, NullifierEvent, TransactionEvent};
use veil_types::{CipherText, DepositCiphertext, UTXO};

use crate::get_key_from_file;

#[derive(Debug, serde::Serialize)]
pub struct Event {
    pub event_type: String,
    pub value: String,
}

impl Event {
    pub fn parse_event(log: &str) -> Option<Event> {
        let parts: Vec<&str> = log.split(" ").collect();
        if parts.len() == 4 {
            return Some(Event {
                event_type: parts[2].to_string(),
                value: parts[3].to_string(),
            });
        }
        None
    }
}

pub fn decrypt_transaction_cipher_text(
    key_path: String,
    value: Vec<u8>,
) -> Result<(Vec<UTXO>, Vec<Vec<u8>>, u64, u64), String> {
    let (spending_key, viewing_key, _deposit_key) = match get_key_from_file(key_path) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };
    let event = match TransactionEvent::try_from_slice(&value) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    if event.commitment_cipher_text.len() != event.commitments.len() {
        return Err("commitments len and cipher text len must be equal".to_string());
    }

    let mut utxos: Vec<UTXO> = vec![];
    let mut leafs: Vec<Vec<u8>> = vec![];
    for idx in 0..event.commitment_cipher_text.len() {
        let text = event.commitment_cipher_text[idx].clone();
        leafs.push(event.commitments[idx].clone());

        let utxo = match UTXO::decrypt(
            CipherText {
                cipher: text.ciphertext,
                nonce: text.nonce,
                blinded_sender_pubkey: text.encrypted_sender_key,
                blinded_receiver_pubkey: text.encrypted_receiver_key,
            },
            viewing_key.clone(),
            spending_key.clone(),
        ) {
            Ok(utxo) => utxo,
            Err(_err) => {
                continue;
            }
        };

        if utxo.utxo_hash() != event.commitments[idx] {
            println!("inserted commitments non valid");
            continue;
        }

        utxos.push(utxo);
    }

    Ok((utxos, leafs, event.tree_number, event.start_position))
}

pub fn decrypt_deposit_cipher_text(
    key_path: String,
    value: Vec<u8>,
) -> Result<(UTXO, u64, u64), String> {
    let (spending_key, viewing_key, _deposit_key) = match get_key_from_file(key_path) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };
    let event = match DepositEvent::try_from_slice(&value) {
        Ok(data) => data,
        Err(err) => return Err(err.to_string()),
    };

    let text = event.shield_cipher_text;
    let utxo = match UTXO::decrypt_for_deposit(
        DepositCiphertext {
            cipher: text.encrypted_text,
            nonce: text.nonce,
            shield_key: text.shield_key,
        },
        event.pre_commitments.token_id,
        event.pre_commitments.value,
        viewing_key,
        spending_key,
    ) {
        Ok(utxo) => utxo,
        Err(err) => {
            return Err(err.to_string());
        }
    };

    Ok((utxo, event.tree_number, event.start_position))
}

pub fn get_nullifiers_from_event(value: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
    let event = match NullifierEvent::try_from_slice(&value) {
        Ok(data) => data,
        Err(err) => return Err(format!("failed to deserialize nullifiers event: {}", err.to_string()))
    };

    Ok(event.nullifiers)
}