use std::collections::HashMap;

use borsh::{BorshSerialize, BorshDeserialize};
use veil_types::{UTXO, MerkleTreeSparse};

pub mod solana;

pub const DEPOSIT_EVENT: &str = "deposit_event";
pub const TRANSFER_EVENT: &str = "transfer_event";
pub const WITHDRAW_EVENT: &str = "withdraw_event";
pub const NULLIFIERS_EVENT: &str = "nullifiers_event";

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct RawData {
    pub tree_data: MerkleTreeSparse<32>,
    pub utxos_data: HashMap<u64, UTXO>,
}

pub struct Data {
    pub data: String, // base64 encode of bytes data
}