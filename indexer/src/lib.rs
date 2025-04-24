use serde::Serialize;
use storage::db::rockdb::StorageWrapper;
use std::{fs, sync::{Arc, RwLock}};

pub mod api_handler;
pub mod client;
pub mod event;
pub mod storage;

const CONTENT_LENGTH: usize = 96;

// Define application state

#[derive(Clone)]
pub struct AppState{
    pub db: Arc<RwLock<StorageWrapper>>,
}

#[derive(Serialize)]
pub struct Data {
    pub data: String, // base64 encode of bytes data
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
