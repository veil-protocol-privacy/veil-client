pub mod db;

pub trait Storage {
    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String>;
    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, String>;
    fn delete(&self, key: Vec<u8>) -> Result<(), String>;
}