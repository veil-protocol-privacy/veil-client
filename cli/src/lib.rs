pub mod commands;
pub mod config;
pub mod storage;
pub mod utils;

pub struct TransferInput {
    pub amount: u64,
    pub merkle_leaf_index: u64,
}

pub struct TransferOutput {
    pub amount: u64,
    // pub receiver_public_viewing_key: String, // re adding this when we want to support batch transfer in one instruction
    pub memo: String,
}
