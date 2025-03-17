use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{ed25519::signature::SignerMut, SigningKey, SECRET_KEY_LENGTH};
use aes_gcm::{aead::Aead, aes::cipher::generic_array::typenum::U12, Aes256Gcm, Key, KeyInit, Nonce};
use crate::{blind_keys, poseidon, share_key, EncryptData};

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

    pub fn sign(
        &self, 
        merkle_root: Vec<u8>,
        params_hash: Vec<u8>,
        nullifiers: Vec<Vec<u8>>,
        output_hashes: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let mut message_data: Vec<&[u8]> = vec![merkle_root.as_slice(), params_hash.as_slice()];
        message_data.extend(nullifiers.iter().map( |nullifier| nullifier.as_slice()));
        message_data.extend(output_hashes.iter().map( |output_hash| output_hash.as_slice()));

        let message_hash = poseidon(message_data);
        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&self.viewing_key);
        let mut signing_key: SigningKey = SigningKey::from_bytes(&secret_key);

        signing_key.sign(&message_hash).to_bytes().to_vec()
    }

    pub fn encrypt(
        self,
        sender_viewing_key: Vec<u8>,
    ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut sender_secret_key = [0u8; SECRET_KEY_LENGTH];
        sender_secret_key.copy_from_slice(&sender_viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&sender_secret_key);
        let sender_viewing_pubkey = signing_key.verifying_key().as_bytes().to_vec();

        let (blinded_sender_pubkey, blinded_receiver_pubkey) = blind_keys(sender_viewing_pubkey, self.viewing_public_key(), self.random.clone());
        let shared_key = share_key(sender_viewing_key, blinded_receiver_pubkey.clone());

        // encrypt data
        let mut encrypt_key: [u8; 32] = [0; 32];
        encrypt_key.copy_from_slice(&shared_key);
        let key = Key::<Aes256Gcm>::from_slice(&encrypt_key);
        let cipher = Aes256Gcm::new(key);
        
        let mut random_bytes = [0u8; 12];
        random_bytes.copy_from_slice(self.random.as_slice());
        let nonce = Nonce::<U12>::from_slice(&random_bytes);

        let encrypt_data = EncryptData {
            master_pubkey: self.master_public_key(),
            random: self.random,
            amount: self.amount,
            token_id: self.token_id,
            memo: self.memo,
        };
        let mut plain_text = Vec::new();
        encrypt_data.serialize(&mut plain_text).unwrap();

        let ciphertext = cipher.encrypt(&nonce, plain_text.as_slice()).unwrap();
        (ciphertext, blinded_sender_pubkey, blinded_receiver_pubkey)
    }
    // decrypt
}

