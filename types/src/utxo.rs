use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{ed25519::signature::SignerMut, SigningKey, SECRET_KEY_LENGTH};
use aes_gcm::{aead::Aead, aes::cipher::generic_array::typenum::U12, Aes256Gcm, Key, KeyInit, Nonce};
use crate::{blind_keys, poseidon, share_key, CipherText, EncryptData};

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
    ) -> CipherText {
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
        random_bytes.copy_from_slice(&self.random[..12]);
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
        CipherText::new(ciphertext, blinded_sender_pubkey, blinded_receiver_pubkey)
    }
    // decrypt
}

#[cfg(test)]
mod tests{
    use aes_gcm::aes::cipher::{self, generic_array::typenum::U32};

    use super::*;

    pub fn encrypt_aes(
        sender_viewing_key: Vec<u8>,
        receiver_viewing_public_key: Vec<u8>,
        random: Vec<u8>,
        data: String,

    ) -> CipherText {
        let mut sender_secret_key = [0u8; SECRET_KEY_LENGTH];
        sender_secret_key.copy_from_slice(&sender_viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&sender_secret_key);
        let sender_viewing_pubkey = signing_key.verifying_key().as_bytes().to_vec();

        let (blinded_sender_pubkey, blinded_receiver_pubkey) = blind_keys(sender_viewing_pubkey, receiver_viewing_public_key, random.clone());
        let shared_key = share_key(sender_viewing_key, blinded_receiver_pubkey.clone());

        // encrypt data
        let mut encrypt_key: [u8; 32] = [0; 32];
        encrypt_key.copy_from_slice(&shared_key);
        let key = Key::<Aes256Gcm>::from_slice(&encrypt_key);
        let cipher = Aes256Gcm::new(key);
        
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&random[..12]);
        let nonce = Nonce::<U12>::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(&nonce, data.as_bytes()).unwrap();
        CipherText::new(ciphertext, blinded_sender_pubkey, blinded_receiver_pubkey)
    }

    #[test]
    fn test_share_keys() {
        let sender_viewing_key: Vec<u8> = vec![38, 114, 103, 252, 36, 91, 2, 181, 87, 194, 26, 61, 225, 16, 23, 253, 224, 129, 71, 180, 18, 140, 156, 215, 1, 182, 243, 148, 162, 107, 157, 15];
        let receiver_viewing_key: Vec<u8> = vec![56, 128, 221, 64, 109, 13, 11, 10, 68, 182, 229, 42, 241, 47, 83, 229, 46, 57, 8, 6, 145, 134, 209, 146, 77, 191, 236, 150, 69, 191, 127, 88];
        let random: Vec<u8> = vec![241, 95, 58, 12, 20, 181, 228, 193, 223, 23, 114, 74, 198, 115, 246, 164, 79, 49, 62, 231, 56, 226, 48, 140, 219, 181, 23, 40, 246, 13, 132, 46];

        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&sender_viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&secret_key);
        let sender_viewing_pubkey = signing_key.verifying_key().as_bytes().to_vec();

        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&receiver_viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&secret_key);
        let receiver_viewing_pubkey = signing_key.verifying_key().as_bytes().to_vec();

        let (blinded_sender_pubkey, blinded_receiver_pubkey) = blind_keys(sender_viewing_pubkey, receiver_viewing_pubkey, random.clone());
        let sender_shared_key = share_key(sender_viewing_key, blinded_receiver_pubkey.clone());
        let receiver_shared_key = share_key(receiver_viewing_key, blinded_sender_pubkey.clone());

       assert_eq!(sender_shared_key, receiver_shared_key)
    }

    #[test]
    fn test_encrypt_decrypt_aes() {
        let sender_viewing_key: Vec<u8> = vec![38, 114, 103, 252, 36, 91, 2, 181, 87, 194, 26, 61, 225, 16, 23, 253, 224, 129, 71, 180, 18, 140, 156, 215, 1, 182, 243, 148, 162, 107, 157, 15];
        let receiver_viewing_key: Vec<u8> = vec![56, 128, 221, 64, 109, 13, 11, 10, 68, 182, 229, 42, 241, 47, 83, 229, 46, 57, 8, 6, 145, 134, 209, 146, 77, 191, 236, 150, 69, 191, 127, 88];
        let random: Vec<u8> = vec![241, 95, 58, 12, 20, 181, 228, 193, 223, 23, 114, 74, 198, 115, 246, 164, 79, 49, 62, 231, 56, 226, 48, 140, 219, 181, 23, 40, 246, 13, 132, 46];

        let mut secret_key = [0u8; SECRET_KEY_LENGTH];
        secret_key.copy_from_slice(&receiver_viewing_key);
        let signing_key: SigningKey = SigningKey::from_bytes(&secret_key);
        let receiver_viewing_pubkey = signing_key.verifying_key().as_bytes().to_vec();

        let cipher_text = encrypt_aes(sender_viewing_key, receiver_viewing_pubkey, random.clone(), String::from("test_encrypt"));

        let shared_key = share_key(receiver_viewing_key, cipher_text.blinded_sender_pubkey);
        
        let mut encrypt_key: [u8; 32] = [0; 32];
        encrypt_key.copy_from_slice(&shared_key);
        let key = Key::<Aes256Gcm>::from_slice(&encrypt_key);
        let cipher = Aes256Gcm::new(key);

        let mut random_bytes = [0u8; 12];
        random_bytes.copy_from_slice(&random[..12]);
        let nonce = Nonce::<U12>::from_slice(&random_bytes);
        let plain_text = match cipher.decrypt(nonce, cipher_text.cipher.as_slice()){
            Ok(plain_text) => plain_text,
            Err(err) => panic!("err: {:#?}", err),
        };

        let decrypt = std::str::from_utf8(&plain_text).unwrap();
        assert_eq!(decrypt, "test_encrypt");
    }
}