use borsh::{BorshDeserialize, BorshSerialize};

pub mod utxo;
pub mod utils;

pub use utils::*;
pub use utxo::*;

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct Arguments {
    pub public_data: PublicData,
    pub private_data: PrivateData,
    pub tree_depth: u64,
    pub input_count: u64,
    pub output_count: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct PublicData {
    pub merkle_root: Vec<u8>,
    pub params_hash: Vec<u8>,
    pub nullifiers: Vec<Vec<u8>>,
    pub output_hashes: Vec<Vec<u8>>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct PrivateData {
    pub token_id: Vec<u8>,
    pub pubkey: Vec<u8>,
    pub signature: Vec<u8>,
    pub random_inputs: Vec<Vec<u8>>,
    pub amount_in: Vec<u64>,
    pub merkle_paths: Vec<Vec<Vec<u8>>>,
    pub merkle_leaf_indices: Vec<u64>,
    pub nullifying_key: Vec<u8>,
    pub utxo_output_keys: Vec<Vec<u8>>,
    pub amount_out: Vec<u64>
}