use crate::{AppState, BalancesResp, LeafsResp, RootResp, UnspentReq};
use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use darksol::{merkle::CommitmentsAccount, utils::serialize::BorshDeserializeWithLength};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};
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

    Json(RootResp {
        root: new_tree.root(),
    })
    .into_response()
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

pub async fn unspent_balances(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UnspentReq>,
) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    match db.get_iterator() {
        Ok(utxos) => {
            let seed = payload.tree_number.to_le_bytes();
            let (commitment_pda, _) =
                Pubkey::try_find_program_address(&[&seed], &state.program_id).unwrap();
            let rpc_client = RpcClient::new_with_commitment(
                state.rpc_url.clone(),
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

            let commitment_account =
                match CommitmentsAccount::<15>::try_from_slice_with_length(&acc_data) {
                    Ok(acc) => acc,
                    Err(e) => {
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("failed to deserialized commitment account data: {}", e),
                        )
                            .into_response();
                    }
                };
            let mut unspent: HashMap<String, u64> = HashMap::new();
            println!("{}", commitment_account.next_leaf_index);

            for (idx, utxo) in utxos {
                if !commitment_account.check_nullifier(&utxo.nullifier(idx)) {
                    let pubkey_array: [u8; 32] = match utxo.token_id().try_into() {
                        Ok(pk) => pk,
                        Err(_) => {
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Expected 32-byte vector"),
                            )
                                .into_response();
                        }
                    };
                    let addr = Pubkey::new_from_array(pubkey_array);
                    *unspent.entry(addr.to_string()).or_insert(0) += utxo.amount();
                }
            }

            return Json(BalancesResp { balances: unspent }).into_response();
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
