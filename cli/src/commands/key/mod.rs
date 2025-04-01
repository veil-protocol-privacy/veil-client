use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use solana_sdk::signer::Signer;

use crate::key::{
    KeyStorage, KeyStorageType,
    raw::{RawKeyStorage, StoredKeypair},
};

#[derive(Clone, Subcommand)]
pub enum KeyCommands {
    Create {
        #[arg(short, long)]
        name: Option<String>,
    },
    Show {
        #[arg(short, long)]
        name: Option<String>,
    },
    List,
}

pub struct KeyConfig {
    path: PathBuf,
    storage: KeyStorageType,
    name: String,
}

impl KeyConfig {
    pub fn new(path: PathBuf, storage: KeyStorageType, name: String) -> Self {
        Self {
            storage,
            path,
            name,
        }
    }
}

impl KeyCommands {
    pub fn handle_command(command: KeyCommands, config: KeyConfig) -> Result<()> {
        let key_storage = match config.storage {
            KeyStorageType::Raw => RawKeyStorage::new(config.path),
            KeyStorageType::Encrypted => unimplemented!(),
        };

        match command {
            KeyCommands::Create { name } => {
                let key_name = name.unwrap_or_else(|| config.name);

                create(key_storage, key_name)
            }
            KeyCommands::Show { name } => {
                let key_name = name.unwrap_or_else(|| config.name);
                show(key_storage, key_name)
            }
            KeyCommands::List => list(key_storage),
        }
    }
}

fn list<T: KeyStorage>(storage: T) -> Result<()> {
    let keys = storage.list_keys()?;
    for key in keys {
        println!("{}", key);
    }
    Ok(())
}

fn create<T: KeyStorage>(storage: T, name: String) -> Result<()> {
    let keypair = StoredKeypair::new();
    storage.save_keypair(&name, &keypair)?;
    println!("Key {} created.", name,);
    Ok(())
}

fn show<T: KeyStorage>(storage: T, name: String) -> Result<()> {
    let keypair = storage.load_keypair(&name)?;
    println!("Loaded key {}: {:?}", name, keypair.deposit_key().pubkey());
    Ok(())
}
