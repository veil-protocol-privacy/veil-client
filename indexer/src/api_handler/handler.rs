use std::sync::Arc;
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use base64::{Engine as _, engine::general_purpose};
use crate::{AppState, Data};

pub async fn roots(State(state): State<Arc<AppState>>, ) -> impl IntoResponse {
    let leafs = state.db.get_iterator_for_tree(tree_number, range)

    Json(Data { data: encode })
}

pub async fn leafs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_iterator() {
        Ok(utxos) => {
            let bytes_data = borsh::to_vec(&utxos).unwrap();
            let encode = general_purpose::STANDARD.encode(bytes_data);

            return Json(Data { data: encode }).into_response();
        },
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch data: {}", e),
            )
                .into_response();
        }
    }
}
