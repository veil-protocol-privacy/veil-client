use base64::{engine::general_purpose, Engine};
use borsh::{BorshDeserialize, BorshSerialize};
use rand::Rng;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use storage::db::rockdb::StorageWrapper;
use veil_types::UTXO;
use std::{collections::HashMap, fs, sync::{Arc, RwLock}};

pub mod api_handler;
pub mod client;
pub mod event;
pub mod storage;

// Define application state
#[derive(Clone)]
pub struct AppState{
    pub db: Arc<RwLock<StorageWrapper>>,
    pub program_id: Pubkey,
    pub key: KeyJson,
    pub rpc_url: String,
}

#[derive(Serialize)]
pub struct RootResp {
    pub root: Vec<u8>
}

#[derive(Serialize)]
pub struct LeafsResp {
    pub utxos: HashMap<u64, UTXO>
}

#[derive(Serialize)]
pub struct BalancesResp {
    pub balances: HashMap<String, u64>
}

#[derive(Serialize, Deserialize)]
pub struct UnspentReq {
    pub tree_number: u64
}

#[derive(Serialize, Deserialize)]
pub struct DepositReq {
    pub amount: u64,
    pub token_id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct TransferMetaData {
    pub amount: u64,
    pub receiver_viewing_pubkey: Vec<u8>,
    pub receiver_master_pubkey: Vec<u8>,
    pub memo: String,
}

#[derive(Serialize, Deserialize)]
pub struct TransferReq {
    pub metas: Vec<TransferMetaData>,
    pub tree_number: u64,
    pub token_id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct WitdrawReq {
    pub receiver_viewing_pubkey: Vec<u8>,
    pub receiver_master_pubkey: Vec<u8>,
    pub amount: u64,
    pub token_id: Vec<u8>,
    pub tree_number: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TxResp {
    pub instruction_data: Vec<u8>,
    pub insert_new_commitment: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct KeyJson {
    pub sk: [u8; 32],
    pub vk: [u8; 32],
    pub dk: [u8; 32],
}

pub fn get_key_from_file(path: String) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    let res = fs::read_to_string("data/output.txt");

    match res {
        Ok(content) => {
            let data =  general_purpose::STANDARD.decode(content).unwrap();
            let key: KeyJson = KeyJson::try_from_slice(&data).unwrap();

            let spending_key = key.sk.to_vec();
            let viewing_key = key.vk.to_vec();
            let deposit_key = key.dk.to_vec();

            Ok((spending_key, viewing_key, deposit_key))
        }
        Err(err) => {
            return Err(format!("cannot read from file: {}", err.to_string(),));
        }
    }
}

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..length).map(|_| rng.random()).collect()
}