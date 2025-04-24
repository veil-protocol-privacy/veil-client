
use std::collections::HashMap;
use veil_types::MerkleTreeSparse;

pub struct MemDb {
    pub tree: MerkleTreeSparse<32>,
}

impl MemDb {
    pub fn new(tree_num: u64) -> Self {
        let tree = MerkleTreeSparse::new(tree_num);

        MemDb {
            tree,
        }
    }

    pub fn insert(&mut self, leafs: Vec<Vec<u8>>) -> HashMap<Vec<u8>, u64> {
        self.tree.insert(leafs)
    }

    pub fn root(&self) -> Vec<u8> {
        self.tree.root()
    }

    pub fn import_tree(&mut self, tree_num: u64, leafs: Vec<Vec<u8>>) -> Self {
        let mut emtpy_tree = MerkleTreeSparse::new(tree_num);   
        emtpy_tree.insert(leafs);

        MemDb { tree: emtpy_tree }
    }
}