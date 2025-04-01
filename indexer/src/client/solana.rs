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

pub struct SolanaClient {
    client: RpcClient,
    ws_client: PubsubClient,
    // db: DbStorage,
    tree: MerkleTreeSparse<32>,
    utxos: HashMap<u64, UTXO>,
}

impl SolanaClient {
    pub async fn new(rpc_url: &str, ws_url: &str) -> Result<Self, Box<dyn Error>> {
        let client = RpcClient::new(rpc_url.to_string());
        let ws_client = PubsubClient::new(ws_url).await?;
        let tree = MerkleTreeSparse::new(0);

        Ok(SolanaClient {
            client,
            ws_client,
            tree,
            utxos: HashMap::new(),
        })
    }

    pub fn insert(&mut self, leafs: Vec<Vec<u8>>) {
        self.tree.insert(leafs);
    }

    pub fn root(&self) -> Vec<u8> {
        self.tree.root()
    }

    pub fn insert_utxo(&mut self, leaf_index: u64, utxo: UTXO) {
        self.utxos.insert(leaf_index, utxo);
    }

    pub async fn listen_to_program_logs(
        &self,
        program_id: Pubkey,
        tx: tokio::sync::mpsc::Sender<Vec<String>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (mut subscription, _) = self
            .ws_client
            .logs_subscribe(
                solana_client::rpc_config::RpcTransactionLogsFilter::Mentions(vec![
                    program_id.to_string(),
                ]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig::finalized()),
                },
            )
            .await?;

        while let Some(logs_result) = subscription.next().await {
            tx.send(logs_result.value.logs).await?;
        }

        Ok(())
    }

    pub async fn fetch_historical_events(
        &self,
        program_id: Pubkey,
        tx: tokio::sync::mpsc::Sender<Vec<String>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let signatures = self.client.get_signatures_for_address(&program_id).await?;

        for signature_info in signatures {
            if let Ok(tx_result) = self
                .client
                .get_transaction(
                    &Signature::from_str(&signature_info.signature).unwrap(),
                    UiTransactionEncoding::Json,
                )
                .await
            {
                // Extract logs from transaction metadata
                if let Some(meta) = &tx_result.transaction.meta {
                    if let logs = &meta.log_messages.clone().unwrap() {
                        tx.send(logs.clone()).await?;
                    }
                }
            }
        }

        Ok(())
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

    // pub fn from_json(&mut self, json_data: Data) -> Self {
    //     let dencoded = general_purpose::STANDARD.decode(json_data.data).unwrap();
    //     let raw_data = RawData::try_from_slice(&dencoded).unwrap();

    //     SolanaClient {
    //         client: self.client,
    //         ws_client: self.ws_client,
    //         tree: raw_data.tree_data,
    //         utxos: raw_data.utxos_data,
    //     }
    // }
}
