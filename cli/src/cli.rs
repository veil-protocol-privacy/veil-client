use crate::{solana::SolanaClient, key::raw::StoredKeypair};

pub struct CliContext {
    pub client: SolanaClient,
    pub program_id: String,
    pub key: StoredKeypair,
}
