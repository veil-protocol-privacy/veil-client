pub mod raw;

use anyhow::Result;
use clap::ValueEnum;
use raw::StoredKeypair;
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyStorageType {
    #[default]
    Raw,
    Encrypted,
}

pub trait KeyStorage {
    fn save_keypair(&self, name: &str, keypair: &StoredKeypair) -> Result<()>;
    fn load_keypair(&self, name: &str) -> Result<StoredKeypair>;
    fn list_keys(&self) -> Result<Vec<String>>;
}
