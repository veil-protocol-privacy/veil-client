use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct TransferInput {
    pub amount: u64,
    pub merkle_leaf_index: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TransferOutput {
    pub amount: u64,
    // pub receiver_public_viewing_key: String, // re adding this when we want to support batch transfer in one instruction
    pub memo: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonContent {
    pub inputs: Vec<TransferInput>,
    pub outputs: Vec<TransferOutput>,
}

pub const CONTENT_LENGTH: usize = 96;

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..length).map(|_| rng.random()).collect()
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
