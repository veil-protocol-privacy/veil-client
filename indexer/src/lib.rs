use axum::Json;
use serde::Serialize;
use std::{collections::HashMap, fs};
use storage::db::memdb::MemDb;
use tokio::sync::Mutex;
use types::UTXO;

pub mod api_handler;
pub mod client;
pub mod event;
pub mod storage;

const CONTENT_LENGTH: usize = 96;

pub type MemState = Mutex<MemDb>;

pub async fn insert(state: MemState, leafs: Vec<Vec<u8>>) -> HashMap<Vec<u8>, u64> {
    let mut db = state.lock().await;
    (*db).insert(leafs)
}

pub async fn root(state: MemState) -> Vec<u8> {
    let db = state.lock().await;
    db.root()
}

pub async fn insert_utxo(state: MemState, leaf_index: u64, utxo: UTXO) {
    let mut db = state.lock().await;
    (*db).insert_utxo(leaf_index, utxo)
}

pub async fn to_json(state: MemState) -> Json<Data> {
    let db = state.lock().await;
    (*db).to_json()
}

// Define application state
pub type AppState = Mutex<String>;

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
