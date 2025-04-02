
use std::collections::HashMap;

use axum::Json;
use base64::{Engine as _, engine::general_purpose};
use client::merkle::MerkleTreeSparse;
use types::UTXO;

use crate::client::RawData;
use crate::Data;

pub struct MemDb {
    tree: MerkleTreeSparse<32>,
    utxos: HashMap<u64, UTXO>,
}

impl MemDb {
    pub fn new() -> Self {
        let tree = MerkleTreeSparse::new(0);

        MemDb {
            tree,
            utxos: HashMap::new(),
        }
    }

    pub fn insert(&mut self, leafs: Vec<Vec<u8>>) -> HashMap<Vec<u8>, u64> {
        self.tree.insert(leafs)
    }

    pub fn root(&self) -> Vec<u8> {
        self.tree.root()
    }

    pub fn insert_utxo(&mut self, leaf_index: u64, utxo: UTXO) {
        self.utxos.insert(leaf_index, utxo);
    }

    pub fn to_json(&self) -> Json<Data> {
        let data = RawData {
            tree_data: self.tree.clone(),
            utxos_data: self.utxos.clone(),
        };

        let data_bytes = borsh::to_vec(&data).unwrap();
        let encoded = general_purpose::STANDARD.encode(&data_bytes);

        Json(Data { data: encoded })
    }
}