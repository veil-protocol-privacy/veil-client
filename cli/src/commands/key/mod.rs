use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{
    config::CliConfig,
    storage::{self, KeyStorage, file::FileKeyStorage},
};

#[derive(Clone, Subcommand)]
pub enum KeyCommand {
    Create { name: String },
    Show { name: String },
    List,
}

pub fn handle_command(command: KeyCommand, config_path: Option<PathBuf>) -> Result<()> {
    let config = CliConfig::load_or_create(config_path)?;
    let key_storage = FileKeyStorage::new(config.key_path.into());

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
