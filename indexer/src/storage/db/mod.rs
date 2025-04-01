use super::Storage;
use rocksdb::{DB, Options};

pub struct DbStorage {
    options: Options,
    path: String,
}

impl Storage for DbStorage {
    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String> {
        let db = DB::open(&self.options, self.path.as_str()).expect("Failed to open RocksDB");

        match db.put(&key, &value) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, String> {
        let db = DB::open(&self.options, self.path.as_str()).expect("Failed to open RocksDB");

        match db.get(&key) {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Err("Key not found".to_string()),
            Err(e) => Err(format!("Error reading from RocksDB: {}", e)),
        }
    
    }

    fn delete(&self, key: Vec<u8>) -> Result<(), String> {
        let db = DB::open(&self.options, self.path.as_str()).expect("Failed to open RocksDB");

        match db.delete(&key) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}


impl DbStorage {
    pub fn new(path: String) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true); // Create DB if it doesn't exist
        DbStorage {
            options: Options::default(),
            path,
        }
    }

    pub fn new_with_options(path: String, opts: Options) -> Self {
        DbStorage { options: opts, path }
    }
}