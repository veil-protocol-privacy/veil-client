use darksol::{merkle::CommitmentsAccount, utils::serialize::BorshDeserializeWithLength};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};
use veil_types::UTXO;

use crate::AppState;

pub mod handler;
pub mod tx;

pub fn get_spentable_utxos(
    state: Arc<AppState>,
    tree_number: u64,
    token_id: Vec<u8>,
    amount: u64,
) -> Result<HashMap<u64, UTXO>, String> {
    let db = state.db.read().unwrap();
    match db.get_iterator() {
        Ok(utxos) => {
            let seed = tree_number.to_le_bytes();
            let (commitment_pda, _) =
                Pubkey::try_find_program_address(&[&seed], &state.program_id).unwrap();
            let rpc_client = RpcClient::new_with_commitment(
                state.rpc_url.clone(),
                CommitmentConfig::confirmed(),
            );
            let acc_data = match rpc_client.get_account_data(&commitment_pda) {
                Ok(data) => data,
                Err(e) => {
                    return Err(e.to_string());
                }
            };

            let commitment_account =
                match CommitmentsAccount::<15>::try_from_slice_with_length(&acc_data) {
                    Ok(acc) => acc,
                    Err(e) => return Err(e.to_string()),
                };
            let mut unspent: HashMap<u64, UTXO> = HashMap::new();
            let mut sum: u64 = 0;

            for (k, utxo) in utxos {
                if !commitment_account.check_nullifier(&utxo.utxo_hash())
                    && utxo.token_id() == token_id
                {
                    unspent.insert(k, utxo.clone());
                    sum += utxo.amount();
                    if sum >= amount {
                        break;
                    }
                }
            }

            return Ok(unspent);
        }
        Err(e) => return Err(e.to_string()),
    }
}
