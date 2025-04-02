use std::collections::HashMap;

use base64::{engine::general_purpose, Engine as _};
use borsh::BorshDeserialize;
use clap::Subcommand;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use types::UTXO;

use crate::cli::CliContext;

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub data: String
}

#[derive(Clone, Debug, Subcommand)]
pub enum IndexerCommands {
    GetUtxo {},

    GetRoot {},
}

impl IndexerCommands {
    pub async fn handle_command(command: IndexerCommands, ctx: &CliContext) {
        match command {
            IndexerCommands::GetUtxo {} => {
                let client = Client::new();
                let response = match client.get(format!("{}/notes", ctx.indexer_api)).send().await {
                    Ok(resp) => resp,
                    Err(err) => return println!("{}", err.to_string()),
                };

                let body = match response.json::<Data>().await {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                };
                
                let decode = match general_purpose::STANDARD.decode(body.data) {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                }; 
                let utxos = match HashMap::<u64, UTXO>::try_from_slice(&decode) {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                };

                println!("{:#?}", utxos)
            }

            IndexerCommands::GetRoot {} => {
                let client = Client::new();
                let response = match client.get(format!("{}/root", ctx.indexer_api)).send().await {
                    Ok(resp) => resp,
                    Err(err) => return println!("{}", err.to_string()),
                };

                let body = match response.json::<Data>().await {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                };
                
                let decode = match general_purpose::STANDARD.decode(body.data) {
                    Ok(data) => data,
                    Err(err) => return println!("{}", err.to_string()),
                }; 
                
                println!("{:?}", decode)
            }
        }
    }
}
