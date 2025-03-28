use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Subcommand, builder::Str};
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::storage::{raw::RawKeyStorage, KeyStorage, KeyStorageType};

#[derive(Clone, Subcommand)]
pub enum KeyCommand {
    Create {
        #[clap(short, long)]
        name: Option<String>,
    },
    Show {
        #[clap(short, long)]
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

pub fn handle_command(command: KeyCommand, config: KeyConfig) -> Result<()> {
    let key_storage = match config.storage {
        KeyStorageType::Raw => RawKeyStorage::new(config.path),
        KeyStorageType::Encrypted => unimplemented!(),
    };

    match command {
        KeyCommand::Create { name } => {
            let key_name = name.unwrap_or_else(|| config.name);

            create(key_storage, key_name)
        }
        KeyCommand::Show { name } => {
            let key_name = name.unwrap_or_else(|| config.name);
            show(key_storage, key_name)
        }
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
