use std::{sync::Arc, vec};
use axum::{Json, extract::State, response::IntoResponse};
use darksol::{
    CommitmentCipherText, DepositRequest, PreCommitments, ShieldCipherText, TransferRequest, WithdrawRequest,
};
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1Stdin};
use veil_types::{generate_utxo_hash, sha256, Arguments, PrivateData, PublicData, UTXO};
use crate::{generate_random_bytes, AppState, DepositReq, TransferReq, TxResp, WitdrawReq};
use super::{get_spentable_utxos, merkle_paths, METHODS_ELF};

pub async fn deposit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DepositReq>,
) -> impl IntoResponse {
    let random: Vec<u8> = generate_random_bytes(32);

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
    let mut leaf_indices: Vec<u64> = vec![];
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut utxo_out_pubkeys: Vec<Vec<u8>> = vec![];
    let mut randoms_in: Vec<Vec<u8>> = vec![];
    let mut amounts_in: Vec<u64> = vec![];
    let mut amounts_out: Vec<u64> = vec![];
    let mut sum_in: u64 = 0;
    let mut sum_out: u64 = 0;
    let random_out = generate_random_bytes(32);
    let nonce = generate_random_bytes(32);

    for tx in payload.metas {
        sum_out += tx.amount;
        amounts_out.push(tx.amount);

        let utxo = UTXO::new(
            vec![], // dont need to input here, as it's private key of the receiver duh :p
            tx.receiver_viewing_pubkey.clone(),
            payload.token_id.clone(),
            random_out.clone(),
            nonce.clone(),
            tx.amount,
            tx.memo.clone(),
        );

        let utxo_hash = generate_utxo_hash(
            random_out.clone(),
            tx.receiver_master_pubkey.clone(),
            payload.token_id.clone(),
            tx.amount,
        );
        commiments.push(utxo_hash);
        utxo_out_pubkeys.push(sha256(vec![&tx.receiver_master_pubkey, &random_out]));

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

    let spenable = match get_spentable_utxos(state.clone(), payload.tree_number, payload.token_id.clone(), sum_out) {
        Ok(balance) => balance,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch balance: {}", e),
            )
                .into_response();
        }
    };

    for (idx, utxo) in spenable.clone() {
        sum_in += utxo.amount();

        let nullifer = utxo.nullifier(idx);
        nullifiers.push(nullifer);
        leaf_indices.push(idx);
        randoms_in.push(utxo.random());
        amounts_in.push(utxo.amount());

        if sum_in >= sum_out {
            break;
        }
    }

    let (merkle_root, merkle_paths) = match merkle_paths(state.clone(), payload.tree_number, leaf_indices.clone()) {
        Ok((root, paths)) => (root, paths),
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch merkle path: {}", e),
            )
                .into_response();
        }
    };
        
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
            random_out.clone(),
            nonce.clone(),
            remain_balance,
            "".to_string(),
        );

        commiments.push(utxo.utxo_hash().clone());
        utxo_out_pubkeys.push(utxo.utxo_public_key());

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
    let utxo_in = spenable.get(&leaf_indices[0]).unwrap();
    let signature = utxo_in.sign(
        merkle_root.clone(),
        vec![],
        nullifiers.clone(),
        commiments.clone(),
    );

    let public_data = PublicData {
        merkle_root: merkle_root.clone(),
        params_hash: vec![],
        nullifiers: nullifiers.clone(),
        output_hashes: commiments.clone(),
    };
    let private_data = PrivateData {
        token_id: payload.token_id.clone(),
        pubkey: utxo_in.spending_public_key(),
        signature,
        random_inputs: randoms_in.clone(),
        amount_in: amounts_in.clone(),
        merkle_paths,
        merkle_leaf_indices: leaf_indices.clone(),
        nullifying_key: utxo_in.nullifying_key(),
        utxo_output_keys: utxo_out_pubkeys,
        amount_out: amounts_out.clone(),
    };

    let args = Arguments {
        public_data,
        private_data,
        tree_depth: 16u64,
        input_count: amounts_in.len() as u64,
        output_count: amounts_out.len() as u64,
    };

    let serialized_args = borsh::to_vec(&args).unwrap();
    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();

    stdin.write_vec(serialized_args);

    // Setup the program for proving.
    let (pk, _vk) = client.setup(METHODS_ELF);

    // Generate the proof
    let proof: SP1ProofWithPublicValues = client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");

    let mut transfer_request = TransferRequest::new(
        proof.bytes(),
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
    let mut leaf_indices: Vec<u64> = vec![];
    let mut commiments: Vec<Vec<u8>> = vec![];
    let mut nullifiers: Vec<Vec<u8>> = vec![];
    let mut commitment_cipher_texts: Vec<CommitmentCipherText> = vec![];
    let mut utxo_out_pubkeys: Vec<Vec<u8>> = vec![];
    let mut randoms_in: Vec<Vec<u8>> = vec![];
    let mut amounts_in: Vec<u64> = vec![];
    let random_out = generate_random_bytes(32);
    let nonce = generate_random_bytes(32);
    let mut insert_new_commitment: bool = false;
    let mut sum_in: u64 = 0;

    let spenable = match get_spentable_utxos(state.clone(), payload.tree_number, payload.token_id.clone(), payload.amount) {
        Ok(balance) => balance,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch balance: {}", e),
            )
                .into_response();
        }
    };

    for (idx, utxo) in spenable.clone() {
        sum_in += utxo.amount();

        let nullifer = utxo.nullifier(idx);
        nullifiers.push(nullifer);

        leaf_indices.push(idx);
        randoms_in.push(utxo.random());
        amounts_in.push(utxo.amount());

        if sum_in >= payload.amount {
            break;
        }
    }

    let (merkle_root, merkle_paths) = match merkle_paths(state.clone(), payload.tree_number, leaf_indices.clone()) {
        Ok((root, paths)) => (root, paths),
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to fetch merkle path: {}", e),
            )
                .into_response();
        }
    };

    let mut new_amounts_out = vec![payload.amount];
    if sum_in > payload.amount {
        let remain_balance = sum_in - payload.amount;
        new_amounts_out.push(remain_balance);

        let utxo = UTXO::new(
            state.key.sk.to_vec(),
            state.key.vk.to_vec(),
            payload.token_id.clone(),
            random_out.clone(),
            nonce.clone(),
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
        utxo_out_pubkeys.push(utxo.utxo_public_key());
        insert_new_commitment = true;
    } else if sum_in < payload.amount {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("insufficient balances"),
        )
            .into_response();
    }

    commiments.push(generate_utxo_hash(
        random_out.clone(),
        payload.receiver_master_pubkey.clone(),
        payload.token_id.clone(),
        payload.amount,
    ));
    utxo_out_pubkeys.push(sha256(vec![&payload.receiver_master_pubkey, &random_out]));

    //------------------------------- Create proof here -------------------------------//
    let utxo_in = spenable.get(&leaf_indices[0]).unwrap();
    let signature = utxo_in.sign(
        merkle_root.clone(),
        vec![],
        nullifiers.clone(),
        commiments.clone(),
    );

    let public_data = PublicData {
        merkle_root: merkle_root.clone(),
        params_hash: vec![],
        nullifiers: nullifiers.clone(),
        output_hashes: commiments.clone(),
    };

    let private_data = PrivateData {
        token_id: payload.token_id.clone(),
        pubkey: utxo_in.spending_public_key(),
        signature,
        random_inputs: randoms_in.clone(),
        amount_in: amounts_in.clone(),
        merkle_paths,
        merkle_leaf_indices: leaf_indices.clone(),
        nullifying_key: utxo_in.nullifying_key(),
        utxo_output_keys: utxo_out_pubkeys,
        amount_out: new_amounts_out.clone(),
    };

    let args = Arguments {
        public_data,
        private_data,
        tree_depth: 16u64,
        input_count: amounts_in.len() as u64,
        output_count: new_amounts_out.len() as u64,
    };
    let serialized_args = borsh::to_vec(&args).unwrap();
    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(serialized_args);
    // Setup the program for proving.
    let (pk, _vk) = client.setup(METHODS_ELF);
    // Generate the proof
    let proof: SP1ProofWithPublicValues = client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");
    
    let mut withdraw_request = WithdrawRequest::new(
        proof.bytes(),
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
