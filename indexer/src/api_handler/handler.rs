use crate::{AppState, Data};
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use base64::{Engine as _, engine::general_purpose};
use std::sync::Arc;
use veil_types::MerkleTreeSparse;

pub async fn roots(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    let leafs = match db.get_iterator_for_tree(0) {
        Ok(val) => val,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch data: {}", err),
            )
                .into_response();
        }
    };

    let inserted_leaf = leafs.iter().map(|(_, v)| v.to_vec()).collect();

    let mut new_tree = MerkleTreeSparse::<32>::new(0);
    new_tree.insert(inserted_leaf);

    let encode = general_purpose::STANDARD.encode(new_tree.root());

    Json(Data { data: encode }).into_response()
}

pub async fn leafs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    match db.get_iterator() {
        Ok(utxos) => {
            let bytes_data = borsh::to_vec(&utxos).unwrap();
            let encode = general_purpose::STANDARD.encode(bytes_data);

            return Json(Data { data: encode }).into_response();
        }
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch data: {}", e),
            )
                .into_response();
        }
    }
}
