pub mod storage;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use solana_sdk::{signature::Keypair, signer::Signer};
use storage::{KeyStorage, KeyStorageType, raw::RawKeyStorage};

#[derive(Clone, Subcommand)]
pub enum KeyCommand {
    Create { name: String },
    Show { name: String },
    List,
}

pub struct KeyConfig {
    path: PathBuf,
    storage: KeyStorageType,
}

impl KeyConfig {
    pub fn new(path: PathBuf, storage: KeyStorageType) -> Self {
        Self { storage, path }
    }
}

pub fn handle_command(command: KeyCommand, config: KeyConfig) -> Result<()> {
    let key_storage = match config.storage {
        KeyStorageType::Raw => RawKeyStorage::new(config.path),
        KeyStorageType::Encrypted => unimplemented!(),
    };

    match command {
        KeyCommand::Create { name } => create(key_storage, name),
        KeyCommand::Show { name } => show(key_storage, name),
        KeyCommand::List => list(key_storage),
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
    let keypair = Keypair::new();
    storage.save_keypair(&name, &keypair)?;
    println!("Key {} created.", name,);
    Ok(())
}

fn show<T: KeyStorage>(storage: T, name: String) -> Result<()> {
    let keypair = storage.load_keypair(&name)?;
    println!("Loaded key {}: {:?}", name, keypair.pubkey());
    Ok(())
}
