use std::sync::Arc;

use axum::Json;
use base64::{engine::general_purpose, Engine as _};
use borsh::BorshDeserialize;
use axum::extract::State;

use crate::{client::RawData, AppState, Data};

fn get_raw_data(data_str: String) -> RawData {
    let decode = general_purpose::STANDARD.decode(data_str).unwrap();
    let raw_data = RawData::try_from_slice(&decode).unwrap();
    raw_data
}

pub async fn roots(State(state): State<Arc<AppState>>) -> Json<Data> {
    let state = state.lock().await;

    let raw_data = get_raw_data(state.clone());
    let root = raw_data.tree_data.root();
    let encode =  general_purpose::STANDARD.encode(root);

    Json(Data{ data: encode })
}

pub async fn leafs(State(state): State<Arc<AppState>>) -> Json<Data> {
    let state = state.lock().await;

    let raw_data = get_raw_data(state.clone());
    let utxos = raw_data.utxos_data;
    let bytes_data = borsh::to_vec(&utxos).unwrap();
    let encode =  general_purpose::STANDARD.encode(bytes_data);

    Json(Data{ data: encode })
}