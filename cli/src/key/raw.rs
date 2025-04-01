use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use super::KeyStorage;

#[derive(Serialize, Deserialize)]
pub struct StoredKeypair {
    pub key: Vec<u8>,
    pub deposit_key: Vec<u8>,
    pub view_key: Vec<u8>,
    pub spend_key: Vec<u8>,
}

pub struct RawKeyStorage {
    path: PathBuf,
}

impl RawKeyStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl StoredKeypair {
    pub fn new() -> Self {
        Self {
            key: Keypair::new().to_bytes().to_vec(),
            deposit_key: Keypair::new().to_bytes().to_vec(),
            view_key: Keypair::new().to_bytes().to_vec(),
            spend_key: Keypair::new().to_bytes().to_vec(),
        }
    }

    pub fn from(key: Keypair) -> Self {
        Self {
            key: key.to_bytes().to_vec(),
            deposit_key: Keypair::new().to_bytes().to_vec(),
            view_key: Keypair::new().to_bytes().to_vec(),
            spend_key: Keypair::new().to_bytes().to_vec(),
        }
    }

    pub fn key(&self) -> Keypair {
        Keypair::from_bytes(&self.key).unwrap()
    }

    pub fn deposit_key(&self) -> Keypair {
        Keypair::from_bytes(&self.deposit_key).unwrap()
    }

    pub fn spend_key(&self) -> Keypair {
        Keypair::from_bytes(&self.spend_key).unwrap()
    }

    pub fn view_key(&self) -> Keypair {
        Keypair::from_bytes(&self.view_key).unwrap()
    }
}

impl KeyStorage for RawKeyStorage {
    fn save_keypair(&self, name: &str, keypair: &StoredKeypair) -> Result<()> {
        let raw_path = self.path.join("raw");
        fs::create_dir_all(&raw_path)?;
        let key_path = raw_path.join(format!("{}.json", name));

        if key_path.exists() {
            println!("Key '{}' already exists at {:?}", name, key_path);
            return Ok(());
        }

        let json = serde_json::to_string(&keypair)?;
        let mut file = File::create(&key_path)?;
        file.write_all(json.as_bytes())?;

        println!("Key {} saved to {}.", name, key_path.display());

        Ok(())
    }

    fn load_keypair(&self, name: &str) -> Result<StoredKeypair> {
        let raw_path = self.path.join("raw");
        fs::create_dir_all(&raw_path)?;
        let key_path = raw_path.join(format!("{}.json", name));
        let mut file = File::open(key_path).context("Key not found")?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let keypair: StoredKeypair = serde_json::from_str(&json)?;
        Ok(keypair)
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let raw_path = self.path.join("raw");
        fs::create_dir_all(&raw_path)?;
        let entries = fs::read_dir(&raw_path)?;
        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry?;
            if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                keys.push(name.to_string());
            }
        }
        Ok(keys)
    }
}
