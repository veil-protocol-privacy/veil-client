use borsh::{BorshSerialize, BorshDeserialize};
use veil_types::MerkleTreeSparse;

pub mod solana;

pub const DEPOSIT_EVENT: &str = "deposit_event";
pub const TRANSFER_EVENT: &str = "transfer_event";
pub const WITHDRAW_EVENT: &str = "withdraw_event";
pub const NULLIFIERS_EVENT: &str = "nullifiers_event";
