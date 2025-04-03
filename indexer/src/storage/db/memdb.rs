use std::{error::Error, str::FromStr};

use axum::Json;
use base64::{Engine as _, engine::general_purpose};
use client::merkle::MerkleTreeSparse;
use futures::StreamExt;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::RpcTransactionLogsConfig,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::collections::HashMap;
use types::UTXO;

use super::RawData;
use crate::Data;

pub struct MemDb {
    tree: MerkleTreeSparse<32>,
    utxos: HashMap<u64, UTXO>,
}

impl MemDb {
    pub async fn new(rpc_url: &str, ws_url: &str) -> Self {
        let tree = MerkleTreeSparse::new(0);

        Ok(SolanaClient {
            tree,
            utxos: HashMap::new(),
        })
    }

    pub fn insert(&mut self, leafs: Vec<Vec<u8>>) -> HashMap<Vec<u8>, u64> {
        self.tree.insert(leafs)
    }

    pub fn root(&self) -> Vec<u8> {
        self.tree.root()
    }

    pub fn insert_utxo(&mut self, leaf_index: u64, utxo: UTXO) {
        self.utxos.insert(leaf_index, utxo);
    }

    pub fn to_json(&self) -> Json<Data> {
        let data = RawData {
            tree_data: self.tree.clone(),
            utxos_data: self.utxos.clone(),
        };

        let data_bytes = borsh::to_vec(&data).unwrap();
        let encoded = general_purpose::STANDARD.encode(&data_bytes);

        Json(Data { data: encoded })
    }
}