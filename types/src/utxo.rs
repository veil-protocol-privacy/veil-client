use borsh::{BorshDeserialize, BorshSerialize};


#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct UTXO {
    spending_key: Vec<u8>,
    viewing_key: Vec<u8>,
    pub amount: u64,
    // TODO: Add more fields
    pub token_data: String,
    pub memo: String,
}

impl UTXO {
    // Nullifying key
    // SpendingPublicKey
    // ViewingPublicKey
    // Master pubkey
    // UTXO pubkey
    // token_id
    // hash
    // sign
    // encrypt
    // decrypt
}