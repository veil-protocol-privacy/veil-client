pub mod raw;

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;

#[derive(ValueEnum, Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyStorageType {
    #[default]
    Raw,
    Encrypted,
}

pub trait KeyStorage {
    fn save_keypair(&self, name: &str, keypair: &Keypair) -> Result<()>;
    fn load_keypair(&self, name: &str) -> Result<Keypair>;
    fn list_keys(&self) -> Result<Vec<String>>;
}
