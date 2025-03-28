use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TransferInput {
    pub amount: u64,
    pub merkle_leaf_index: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TransferOutput {
    pub amount: u64,
    // pub receiver_public_viewing_key: String, // re adding this when we want to support batch transfer in one instruction
    pub memo: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonContent {
    pub inputs: Vec<TransferInput>,
    pub outputs: Vec<TransferOutput>
}