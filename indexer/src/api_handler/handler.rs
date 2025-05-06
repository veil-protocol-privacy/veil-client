use crate::{AppState, LeafsResp, RootResp, UnspentReq};
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use darksol::{merkle::CommitmentsAccount, utils::serialize::BorshDeserializeWithLength};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};
use veil_types::{MerkleTreeSparse, UTXO};

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

    Json(RootResp { root: new_tree.root() }).into_response()
}

pub async fn leafs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    match db.get_iterator() {
        Ok(utxos) => {
            return Json(LeafsResp { utxos }).into_response();
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

pub async fn unspent_balances(State(state): State<Arc<AppState>>, Json(payload): Json<UnspentReq>) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    match db.get_iterator() {
        Ok(utxos) => {
            let seed = payload.tree_number.to_le_bytes();
            let (commitment_pda, _) = Pubkey::try_find_program_address(&[&seed], &state.program_id).unwrap();
            let rpc_client = RpcClient::new_with_commitment(
                String::from("http://127.0.0.1:8899"),
                CommitmentConfig::confirmed(),
            );  
            let acc_data = match rpc_client.get_account_data(&commitment_pda) {
                Ok(data) => data,
                Err(e) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to fetch commitment account data: {}", e),
                    )
                        .into_response();
                }
            };

            let commitment_account = match CommitmentsAccount::<15>::try_from_slice_with_length(&acc_data) {
                Ok(acc) => acc,
                Err(e) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to deserialized commitment account data: {}", e),
                    )
                        .into_response();
                }
            };
            let mut unspent: HashMap<u64, UTXO> = HashMap::new();

            for (k, utxo) in utxos {
                if !commitment_account.check_nullifier(&utxo.utxo_hash()) {
                    unspent.insert(k,utxo);
                }
            }
            
            return Json(LeafsResp { utxos: unspent }).into_response();
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
