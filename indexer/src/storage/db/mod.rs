use memdb::MemDb;
use rockdb::StorageWrapper;
use veil_types::MerkleTreeSparse;

pub mod memdb;
pub mod rockdb;

pub struct Storage {
    pub rockdb: StorageWrapper,
    pub memdb: MemDb,
}

impl Storage {
    pub fn root(&mut self, tree_num: u64) -> Result<Vec<u8>, String> {
        if self.memdb.tree.next_leaf_index > 0 {
            let root = self.memdb.root();

            return Ok(root);
        }

        let leafs = match self.rockdb.get_iterator_for_tree(tree_num) {
            Ok(val) => val,
            Err(err) => return Err(err),
        };

        let inserted_leaf = leafs.iter().map(|(k, v)| v.to_vec()).collect();

        let mut new_tree = MerkleTreeSparse::<32>::new(tree_num);
        new_tree.insert(inserted_leaf);

        // cache the tree
        self.memdb = MemDb {
            tree: new_tree.clone(),
        };

        Ok(new_tree.root())
    }

    pub fn cache_tree(&mut self, tree_num: u64) -> Result<(), String> {
        let leafs = match self.rockdb.get_iterator_for_tree(tree_num) {
            Ok(val) => val,
            Err(err) => return Err(err),
        };

        let inserted_leaf = leafs.iter().map(|(_, v)| v.to_vec()).collect();

        let mut new_tree = MerkleTreeSparse::<32>::new(tree_num);
        new_tree.insert(inserted_leaf);

        // cache the tree
        self.memdb = MemDb {
            tree: new_tree.clone(),
        };

        Ok(())
    }

    pub fn insert_leaf(
        &mut self,
        tree_num: u64,
        leaf_index: u64,
        leaf: Vec<u8>,
    ) -> Result<(), String> {
        if self.memdb.tree.next_leaf_index > 0 {
            self.memdb.tree.insert(vec![leaf.clone()]);
        }

        self.rockdb.insert_leafs(tree_num, leaf_index, leaf)
    }
}
