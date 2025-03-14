use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};

use crate::poseidon;

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct UTXO {
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    random: Vec<u8>,
    token_id: Vec<u8>,
    amount: u64,
    memo: String,
}

impl UTXO {
    pub fn new(spending_key: Vec<u8>, viewing_key: Vec<u8>, token_id: Vec<u8>, random: Vec<u8>, amount: u64, memo: String) -> Self {
        Self {
            spending_key,
            viewing_key,
            random,
            token_id,
            amount,
            memo,
        }
    }

    pub fn nullifying_key(&self) -> Vec<u8> {
        poseidon(vec![self.viewing_key.as_slice()])
    }

    pub fn spending_public_key(&self) -> Vec<u8> {
        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&self.spending_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&secret_key);
        signing_key.verifying_key().as_bytes().to_vec()
    }

    pub fn viewing_public_key(&self) -> Vec<u8> {
        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&self.viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&secret_key);
        signing_key.verifying_key().as_bytes().to_vec()
    }

    pub fn master_public_key(&self) -> Vec<u8> {
        let spending_key = self.spending_public_key();
        let viewing_key = self.viewing_public_key();
        poseidon(vec![spending_key.as_slice(), viewing_key.as_slice()])
    }

    pub fn utxo_public_key(&self) -> Vec<u8> {
        let spending_key = self.spending_public_key();

        poseidon(vec![spending_key.as_slice(), self.random.as_slice()])
    }

    pub fn nullifier(&self, leaf_index: u64) -> Vec<u8> {
        let nullifying_key = self.nullifying_key();
        let leaf_index_bytes = leaf_index.to_le_bytes().to_vec();
        poseidon(vec![nullifying_key.as_slice(), leaf_index_bytes.as_slice()])
    }

    pub fn utxo_hash(&self) -> Vec<u8> {
        let uxto_pubkey = self.utxo_public_key();
        let token_id = self.token_id.clone();
        let amount = self.amount.to_le_bytes().to_vec();

        poseidon(vec![uxto_pubkey.as_slice(), token_id.as_slice(), amount.as_slice()])
    }

    // sign
    // encrypt
    // decrypt
}