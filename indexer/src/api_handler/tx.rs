use std::sync::Arc;
use axum::{Json, extract::State, response::IntoResponse};
use darksol::{
    CommitmentCipherText, DepositRequest, PreCommitments, ShieldCipherText, TransferRequest, WithdrawRequest,
};
use veil_types::{UTXO, generate_utxo_hash};
use crate::{generate_random_bytes, AppState, DepositReq, TransferReq, TxResp, WitdrawReq};
use super::get_spentable_utxos;

pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DepositReq>,
) -> impl IntoResponse {
    let random = generate_random_bytes(32);

    let utxo = UTXO::new(
        state.key.sk.to_vec().clone(),
        state.key.vk.to_vec().clone(),
        payload.token_id,
        random.clone(),
        generate_random_bytes(32),
        payload.amount,
        "".to_string(),
    );

    let pre_commitment =
        PreCommitments::new(utxo.amount(), utxo.token_id(), utxo.utxo_public_key());
    let deposit_ciphertext =
        utxo.encrypt_for_deposit(random.clone(), state.key.dk.to_vec().clone());

    let shield_cipher_text = ShieldCipherText::new(
        deposit_ciphertext.shield_key,
        deposit_ciphertext.cipher,
        utxo.nonce(),
    );

    let request = DepositRequest::new(pre_commitment, shield_cipher_text);
    let instruction_data = match borsh::to_vec(&request) {
        Ok(data) => data,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialized deposit request: {}", err.to_string()),
            )
                .into_response();
        }
    };

    Json(TxResp { instruction_data, insert_new_commitment: true }).into_response()
}

pub async fn transfer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TransferReq>,
) -> impl IntoResponse {
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut sum_in: u64 = 0;
    let mut sum_out: u64 = 0;

    for tx in payload.metas {
        sum_out += tx.amount;
        let random = generate_random_bytes(32);

        let utxo = UTXO::new(
            vec![], // dont need to input here, as it's private key of the receiver duh :p
            tx.receiver_viewing_pubkey.clone(),
            payload.token_id.clone(),
            random.clone(),
            generate_random_bytes(32),
            tx.amount,
            tx.memo.clone(),
        );

        let utxo_hash = generate_utxo_hash(
            random.clone(),
            tx.receiver_master_pubkey,
            payload.token_id.clone(),
            tx.amount,
        );
        commiments.push(utxo_hash);

        let cipher_text = utxo.encrypt(state.key.vk.to_vec().clone());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce().clone(),
            tx.memo.as_bytes().to_vec().clone(),
        );
        commitment_cipher_texts.push(commitment_cipher_text);
    }

    let spenable = match get_spentable_utxos(state, payload.tree_number, payload.token_id.clone(), sum_out) {
        Ok(balance) => balance,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch balance: {}", e),
            )
                .into_response();
        }
    };

    for (idx, utxo) in spenable {
        sum_in += utxo.amount();

        let nullifer = utxo.nullifier(idx);
        nullifiers.push(nullifer);
    }


    // check total input and output
    // if input < output then it is insurficent balance and should return an error
    // if input > output then add a new UTXO for sender represent their new balance
    if sum_in < sum_out {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "total inputs less than total outputs".to_string(),
        )
            .into_response();
    } else if sum_in > sum_out{
        let remain_balance = sum_in - sum_out;

        let utxo = UTXO::new(
            state.key.sk.to_vec(),
            state.key.vk.to_vec(),
            payload.token_id.clone(),
            generate_random_bytes(32),
            generate_random_bytes(32),
            remain_balance,
            "".to_string(),
        );

        commiments.push(utxo.utxo_hash().clone());

        let cipher_text = utxo.encrypt(state.key.vk.to_vec().clone());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce(),
            "".to_string().as_bytes().to_vec(),
        );
        commitment_cipher_texts.push(commitment_cipher_text);
    }

    //------------------------------- Create proof here -------------------------------//

    let mut transfer_request = TransferRequest::new(
        proof,
        merkle_root,
        payload.tree_number,
        commitment_cipher_texts,
    );

    nullifiers.iter().for_each(|nullifier| {
        transfer_request.push_nullifiers(nullifier.clone());
    });

    commiments.iter().for_each(|commitment| {
        transfer_request.push_encrypted_commitments(commitment.clone());
    });

    let instruction_data = match borsh::to_vec(&transfer_request) {
        Ok(data) => data,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialized deposit request: {}", err.to_string()),
            )
                .into_response();
        }
    };

    Json(TxResp { instruction_data, insert_new_commitment: true }).into_response()
}


pub async fn withdraw(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WitdrawReq>,
) -> impl IntoResponse {
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut sum_in: u64 = 0;
    let insert_new_commitment: bool = false;

    let spenable = match get_spentable_utxos(state, payload.tree_number, payload.token_id.clone(), payload.amount) {
        Ok(balance) => balance,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch balance: {}", e),
            )
                .into_response();
        }
    };

    for (idx, utxo) in spenable {
        sum_in += utxo.amount();

        let nullifer = utxo.nullifier(idx);
        nullifiers.push(nullifer);
    }

    if sum_in > payload.amount {
        let remain_balance = sum_in - payload.amount;

        let utxo = UTXO::new(
            state.key.sk.to_vec(),
            state.key.vk.to_vec(),
            payload.token_id.clone(),
            generate_random_bytes(32),
            generate_random_bytes(32),
            remain_balance,
            "".to_string(),
        );

        let commiment = utxo.utxo_hash().clone();
        commiments.push(commiment);

        let cipher_text = utxo.encrypt(state.key.vk.to_vec());
        let commitment_cipher_text = CommitmentCipherText::new(
            cipher_text.blinded_sender_pubkey,
            cipher_text.cipher,
            cipher_text.blinded_receiver_pubkey,
            utxo.nonce(),
            "".to_string().as_bytes().to_vec(),
        );

        commitment_cipher_texts.push(commitment_cipher_text);
        insert_new_commitment = true;
    } else if sum_in < payload.amount {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("insufficient balances"),
        )
            .into_response();
    }

    let mut withdraw_request = WithdrawRequest::new(
        proof,
        merkle_root,
        payload.tree_number,
        payload.amount,
        payload.token_id.clone(),
        commitment_cipher_texts,
    );

    nullifiers.iter().for_each(|nullifier| {
        withdraw_request.push_nullifiers(nullifier.clone());
    });

    commiments.iter().for_each(|commitment| {
        withdraw_request.push_encrypted_commitment(commitment.clone());
    });

    let instruction_data = match borsh::to_vec(&withdraw_request) {
        Ok(data) => data,
        Err(err) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialized withdraw request: {}", err.to_string()),
            )
                .into_response();
        },
    };

    Json(TxResp { instruction_data, insert_new_commitment }).into_response()
}
