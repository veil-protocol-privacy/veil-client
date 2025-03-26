use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{Keypair, Signer};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use super::KeyStorage;

#[derive(Serialize, Deserialize)]
struct StoredKeypair {
    secret: Vec<u8>,
}

pub struct RawKeyStorage {
    path: PathBuf,
}

impl RawKeyStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl KeyStorage for RawKeyStorage {
    fn save_keypair(&self, name: &str, keypair: &Keypair) -> Result<()> {
        let raw_path = self.path.join("raw");
        fs::create_dir_all(&raw_path)?;
        let key_path = raw_path.join(format!("{}.json", name));
        let stored = StoredKeypair {
            secret: keypair.to_bytes().to_vec(),
        };
        let json = serde_json::to_string(&stored)?;
        let mut file = File::create(&key_path)?;
        file.write_all(json.as_bytes())?;

        println!("Key {} saved to {}.", name, key_path.display());

        Ok(())
    }

    fn load_keypair(&self, name: &str) -> Result<Keypair> {
        let raw_path = self.path.join("raw");
        fs::create_dir_all(&raw_path)?;
        let key_path = raw_path.join(format!("{}.json", name));
        let mut file = File::open(key_path).context("Key not found")?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let stored: StoredKeypair = serde_json::from_str(&json)?;
        let keypair = Keypair::from_bytes(&stored.secret)?;
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
