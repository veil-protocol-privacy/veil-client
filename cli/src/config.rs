use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::key::KeyStorageType;

const DEFAULT_RPC_URL: &str = "http://localhost:8899";
const DEFAULT_KEY_STORAGE: KeyStorageType = KeyStorageType::Raw;
const DEFAULT_KEY: &str = "id";

#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub key_path: String,
    pub key_storage: KeyStorageType,
    pub key: String,

    pub rpc_url: String,
}

impl CliConfig {
    fn config_path(path: Option<PathBuf>) -> PathBuf {
        if let Some(path) = path {
            path.clone()
        } else {
            Self::default_path()
        }
    }

    pub fn load_or_create(path: Option<PathBuf>) -> Result<Self> {
        let path = Self::config_path(path);
        if path.exists() {
            // Load existing configuration
            let config_content = fs::read_to_string(&path)
                .context(format!("Failed to read config file at {:?}", path))?;
            serde_yaml::from_str(&config_content)
                .with_context(|| format!("Failed to parse config file at {:?}", path))
        } else {
            // Create default configuration
            let default_config = Self::default();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create config directory at {:?}", parent)
                })?;
            }
            let config_content = serde_yaml::to_string(&default_config)
                .context("Failed to serialize default configuration")?;
            fs::write(&path, config_content)
                .with_context(|| format!("Failed to write default config file at {:?}", path))?;
            Ok(default_config)
        }
    }

    fn default_path() -> PathBuf {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("veil/cli/config.yml");
        path
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            key_path: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("veil/keys")
                .to_string_lossy()
                .into_owned(),
            key_storage: DEFAULT_KEY_STORAGE,
            key: DEFAULT_KEY.to_string(),

            rpc_url: DEFAULT_RPC_URL.to_string(),
        }
    }
}
