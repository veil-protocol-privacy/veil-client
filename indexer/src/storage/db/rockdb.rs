use std::collections::HashMap;
use borsh::BorshDeserialize;
use rocksdb::{
    ColumnFamilyDescriptor, DBWithThreadMode, IteratorMode, MultiThreaded, Options,
};
use veil_types::utxo::UTXO;

pub struct RockDbStorage<const ENABLE_MERKLE_INDEX: bool> {
    pub db: DBWithThreadMode<MultiThreaded>,
}

pub struct LeafRange {
    pub start: u64,
    pub end: u64,
}

impl<const ENABLE_MERKLE_INDEX: bool> RockDbStorage<ENABLE_MERKLE_INDEX> {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let mut cfs = vec![];

        if ENABLE_MERKLE_INDEX {
            cfs.push(ColumnFamilyDescriptor::new("merkle", Options::default()));
        }

        cfs.push(ColumnFamilyDescriptor::new("utxos", Options::default()));

        let db = DBWithThreadMode::<MultiThreaded>::open_cf_descriptors(&opts, path, cfs).unwrap();
        RockDbStorage { db }
    }

    pub fn insert_utxo(
        &mut self,
        tree_number: u64,
        leaf_index: u64,
        utxo: UTXO,
    ) -> Result<(), String> {
        let key = get_key(tree_number, leaf_index);
        let utxo_cf = self.db.cf_handle("utxos").unwrap();

        let value = match borsh::to_vec(&utxo) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "fail to serialized UTXO: {}",
                    e.to_string(),
                ));
            }
        };

        self.db
            .put_cf(&utxo_cf, key, value)
            .map_err(|err| err.into_string())
    }

    pub fn get_utxo(&self, tree_number: u64, leaf_index: u64) -> Result<UTXO, String> {
        let key = get_key(tree_number, leaf_index);
        let utxo_cf = self.db.cf_handle("utxos").unwrap();

        match self.db.get_cf(&utxo_cf, key) {
            Ok(value) => {
                if Some(value.clone()).is_some() {
                    let utxo = match UTXO::try_from_slice(&value.unwrap()) {
                        Ok(v) => v,
                        Err(e) => {
                            return Err(format!(
                                "fail to deserialized UTXO: {}",
                                e.to_string(),
                            ));
                        }
                    };

                    Ok(utxo)
                } else {
                    Err(format!(
                        "no hash found for leaf at {} of tree {}",
                        leaf_index, tree_number
                    ))
                }
            }
            Err(err) => Err(err.into_string()),
        }
    }

    pub fn delete_utxo(&mut self, tree_number: u64, leaf_index: u64) -> Result<(), String> {
        let key = get_key(tree_number, leaf_index);
        let utxo_cf = self.db.cf_handle("utxos").unwrap();

        self.db
            .delete_cf(&utxo_cf, key)
            .map_err(|err| err.into_string())
    }

    pub fn get_iterator(
        &self,
    ) -> Result<Vec<UTXO>, String> {
        let utxo_cf = self.db.cf_handle("utxos").unwrap();

        let iter = self.db.iterator_cf(&utxo_cf, IteratorMode::End);
        let mut map: Vec<UTXO> = Vec::new();

        for (_, value) in iter.filter_map(Result::ok) {
            if Some(value.clone()).is_some() {
                let utxo = match UTXO::try_from_slice(&value) {
                    Ok(val) => val,
                    Err(_) => continue,
                };

                map.push(utxo);
            }
        }
        Ok(map)
    }
}

impl RockDbStorage<true> {
    pub fn insert_leaf(
        &mut self,
        tree_number: u64,
        leaf_index: u64,
        hash: Vec<u8>,
    ) -> Result<(), String> {
        let key = get_key(tree_number, leaf_index);
        let merkle_cf = self.db.cf_handle("merkle").unwrap();

        self.db
            .put_cf(&merkle_cf, key, hash)
            .map_err(|err| err.into_string())
    }

    pub fn get_leaf(&self, tree_number: u64, leaf_index: u64) -> Result<Vec<u8>, String> {
        let key = get_key(tree_number, leaf_index);
        let merkle_cf = self.db.cf_handle("merkle").unwrap();

        match self.db.get_cf(&merkle_cf, key) {
            Ok(value) => {
                if Some(value.clone()).is_some() {
                    Ok(value.unwrap())
                } else {
                    Err(format!(
                        "no hash found for leaf at {} of tree {}",
                        leaf_index, tree_number
                    ))
                }
            }
            Err(err) => Err(err.into_string()),
        }
    }

    pub fn delete_leaf(&mut self, tree_number: u64, leaf_index: u64) -> Result<(), String> {
        let key = get_key(tree_number, leaf_index);
        let merkle_cf = self.db.cf_handle("merkle").unwrap();

        self.db
            .delete_cf(&merkle_cf, key)
            .map_err(|err| err.into_string())
    }

    pub fn get_iterator_for_tree(
        &self,
        tree_number: u64,
    ) -> Result<HashMap<Vec<u8>, Vec<u8>>, String> {
        let merkle_cf = self.db.cf_handle("merkle").unwrap();
        let prefix = format!("tree{}-", tree_number).as_bytes().to_vec();

        let iter = self.db.prefix_iterator_cf(&merkle_cf, prefix);
        let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

        for (key, value) in iter
            .filter_map(Result::ok)
        {
            if Some(value.clone()).is_some() {
                map.insert(key.to_vec(), value.to_vec());
            }
        }
        Ok(map)
    }

    pub fn get_iterator_for_tree_with_range(
        &self,
        tree_number: u64,
        range: LeafRange,
    ) -> Result<HashMap<Vec<u8>, Vec<u8>>, String> {
        let merkle_cf = self.db.cf_handle("merkle").unwrap();
        let prefix = format!("tree{}-", tree_number).as_bytes().to_vec();

        let iter = self.db.prefix_iterator_cf(&merkle_cf, prefix);
        let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

        for (key, value) in iter
            .filter_map(Result::ok)
            .take_while(|(k, _)| k.iter().as_slice() <= get_key(tree_number, range.end).as_slice())
        {
            if Some(value.clone()).is_some() {
                map.insert(key.to_vec(), value.to_vec());
            }
        }
        Ok(map)
    }
}

pub enum StorageWrapper {
    WithMerkle(RockDbStorage<true>),
    WithoutMerkle(RockDbStorage<false>),
}

impl StorageWrapper {
    pub fn get_leaf(&self, tree_number: u64, leaf_index: u64) -> Result<Vec<u8>, String>  {
        match self {
            StorageWrapper::WithMerkle(s) => s.get_leaf(tree_number, leaf_index),
            StorageWrapper::WithoutMerkle(_) => Err("indexer not supports this api".to_string()),
        }
    }

    pub fn get_utxo(&self, tree_number: u64, leaf_index: u64) -> Result<UTXO, String>  {
        match self {
            StorageWrapper::WithMerkle(s) => s.get_utxo(tree_number, leaf_index),
            StorageWrapper::WithoutMerkle(s) => s.get_utxo(tree_number, leaf_index),
        }
    }

    pub fn insert_leafs(&mut self, tree_number: u64, leaf_index: u64, value: Vec<u8>,) -> Result<(), String> {
        match self {
            StorageWrapper::WithMerkle(s) => s.insert_leaf(tree_number, leaf_index, value),
            StorageWrapper::WithoutMerkle(_) => Err("indexer not supports this api".to_string()),
        }
    }

    pub fn insert_utxo(&mut self, tree_number: u64, leaf_index: u64, value: UTXO) -> Result<(), String> {
        match self {
            StorageWrapper::WithMerkle(s) => s.insert_utxo(tree_number, leaf_index, value),
            StorageWrapper::WithoutMerkle(s) => s.insert_utxo(tree_number, leaf_index, value),
        }
    }

    pub fn get_iterator_for_tree(&self, tree_number: u64) -> Result<HashMap<Vec<u8>, Vec<u8>>, String>  {
        match self {
            StorageWrapper::WithMerkle(s) => s.get_iterator_for_tree(tree_number),
            StorageWrapper::WithoutMerkle(_) => Err("indexer not supports this api".to_string()),
        }
    }

    pub fn get_iterator_for_tree_with_range(&self, tree_number: u64, range: LeafRange) -> Result<HashMap<Vec<u8>, Vec<u8>>, String>  {
        match self {
            StorageWrapper::WithMerkle(s) => s.get_iterator_for_tree_with_range(tree_number, range),
            StorageWrapper::WithoutMerkle(_) => Err("indexer not supports this api".to_string()),
        }
    }

    pub fn get_iterator(&self) -> Result<Vec<UTXO>, String>  {
        match self {
            StorageWrapper::WithMerkle(s) => s.get_iterator(),
            StorageWrapper::WithoutMerkle(s) => s.get_iterator(),
        }
    }
}

pub fn get_key(tree_number: u64, leaf_index: u64) -> Vec<u8> {
    return format!("tree{}-leaf{}", tree_number, leaf_index)
        .as_bytes()
        .to_vec();
}
