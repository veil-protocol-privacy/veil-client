pub mod file;

use anyhow::Result;
use solana_sdk::signature::Keypair;

pub trait KeyStorage {
    fn save_keypair(&self, name: &str, keypair: &Keypair) -> Result<()>;
    fn load_keypair(&self, name: &str) -> Result<Keypair>;
    fn list_keys(&self) -> Result<Vec<String>>;
}
