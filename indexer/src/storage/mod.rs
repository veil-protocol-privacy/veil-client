pub mod db;

const DEFAULT_PATH: &str = "../../data/rockdb";

pub struct DbOptions {
    pub path: String,
    pub enable_merkle_indexing: bool,
}

impl DbOptions {
    pub fn new(path: String, enable_merkle_indexing: bool) -> Self {
        DbOptions { path, enable_merkle_indexing }
    }

    pub fn default() -> Self {
        DbOptions { path: DEFAULT_PATH.to_string(), enable_merkle_indexing: true }
    }
}
