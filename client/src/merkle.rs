use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::U256;
use types::hash_left_right;

// Merkle Tree Sparse for scan and find tree path
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct MerkleTreeSparse<const TREE_DEPTH: usize> {
    pub next_leaf_index: usize,
    tree_number: u64,
    zeros: Vec<Vec<u8>>,
    tree: Vec<Vec<Vec<u8>>>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct MerkleProof {
    pub index: u64,
    pub element: Vec<u8>,
    pub path: Vec<Vec<u8>>,
    pub root: Vec<u8>
}

impl<const TREE_DEPTH: usize> MerkleTreeSparse<TREE_DEPTH> {

    /// Create a new empty Merkle Tree
    pub fn new(tree_number: u64) -> Self {
        let zero_value = u256_to_bytes(ZERO_VALUE).to_vec();
        let mut zeros: Vec<Vec<u8>> = Vec::with_capacity(TREE_DEPTH);
        zeros.push(zero_value.clone());

        let mut tree: Vec<Vec<Vec<u8>>> = Vec::with_capacity(TREE_DEPTH);
        tree.push(vec![]);

        let mut current_zero = hash_left_right(zero_value.clone(), zero_value.clone());
        for _ in 1..TREE_DEPTH {
            // Push it to zeros array
            zeros.push(current_zero.clone());

            tree.push(vec![current_zero.clone()]);

            // Calculate the zero value for this level
            current_zero = hash_left_right(current_zero.clone(), current_zero.clone());
        }

        Self {
            next_leaf_index: 0,
            tree_number,
            zeros,
            tree,
        }
    }

    pub fn insert(
        &mut self,
        leaf_nodes: Vec<Vec<u8>>,
    ) {
        if leaf_nodes.len() == 0 {
            return;
        }

        leaf_nodes.iter().for_each(| leaf | {
            self.tree[0].push(leaf.clone());
        });

        self.next_leaf_index += leaf_nodes.len();
        self.rebuild_sparse_tree();
    }

    fn rebuild_sparse_tree(
        &mut self
    ) {
        for level in 0..TREE_DEPTH-1 {
            self.tree[level+1].clear();
            let level_subtree = self.tree[level].clone();
            for pos in (0..level_subtree.len()).step_by(2) {
                if level_subtree.len() - 1 == pos {
                    self.tree[level+1].push(
                        hash_left_right(level_subtree[pos].clone(), self.zeros[level].clone())
                    );
                } else {
                    self.tree[level+1].push(
                        hash_left_right(level_subtree[pos].clone(), level_subtree[pos + 1].clone())
                    );
                }
            }
        }
    }

    pub fn generate_proof(
        &self,
        element: Vec<u8>
    ) -> MerkleProof {
        let mut path: Vec<Vec<u8>> = Vec::with_capacity(TREE_DEPTH);

        let find = self.tree[0].iter().position(| leaf| leaf.eq(&element));
        if find.is_none() {
            panic!("element not in merkle tree")
        }

        let mut index = find.unwrap();
        for level in 0..TREE_DEPTH-1 {
            if index % 2 == 0 {
                if self.tree[level].len() - 1 <= index {
                    path.push(self.zeros[level].clone());
                } else {
                    path.push(self.tree[level][index+1].clone());
                }
            } else {
                path.push(self.tree[level][index-1].clone());
            }
            index = index / 2;
        }

        MerkleProof { 
            index: find.unwrap() as u64,
            element,
            path,
            root: self.root()
        }
    }

    /// Get the Merkle root
    pub fn root(&self) -> Vec<u8> {
        self.tree[TREE_DEPTH -1][0].clone()
    }
}

pub const ZERO_VALUE: U256 = U256([
    0x30644E72E131A029,
    0xB85045B68181585D,
    0x2833E84879B97091,
    0x1A0111EA397FE69A,
]);

pub fn u256_to_bytes(value: U256) -> [u8; 32] {
    let mut bytes: [u8; 32] = [0u8; 32];
    value.to_big_endian(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet_crypto::{Felt, poseidon_hash_many};

    pub fn poseidon(
        inputs: Vec<&[u8]>
    ) -> Vec<u8> {
        let inputs = inputs.iter().map(|input| {
            let mut bytes = [0u8; 32];
            if input.len() < 32 {
                // fill from the last index
                let start = 32 - input.len();
                bytes[start..].copy_from_slice(&input[..]);
            } else {
                bytes.copy_from_slice(input);
            };
            Felt::from_bytes_be(&bytes)
        }).collect::<Vec<Felt>>();
        Vec::from(poseidon_hash_many(inputs.as_slice()).to_bytes_be())
    }

    #[test]
    fn test_zero_tree() {
        let zero_value = u256_to_bytes(ZERO_VALUE).to_vec();
        const TREE_DEPTH: usize = 8;
        let zero_tree = MerkleTreeSparse::<TREE_DEPTH>::new(0);

        assert_eq!(zero_tree.zeros[0], zero_value.clone());
        let mut level_zero = zero_value.clone();
        for i in 1..TREE_DEPTH {
            level_zero = hash_left_right(level_zero.clone(), level_zero.clone());
            assert_eq!(zero_tree.zeros[i], level_zero);
            assert_eq!(zero_tree.tree[i], vec![level_zero.clone()]);
        }

        assert_eq!(zero_tree.root(), level_zero);
    }
    
    #[test]
    fn test_insert() {
        const TREE_DEPTH: usize = 5;

        let mut gap = 1;
        let mut root_lists = vec![];
        while gap < 10 {
            let mut tree = MerkleTreeSparse::<TREE_DEPTH>::new(0);
            let root = tree.root();

            for step in 0..(16 / gap) {
                let mut insert_list = vec![];
                for i in (step * gap)..((step + 1) * gap) {
                    let hash_i = poseidon(vec![&[i]]);
                    insert_list.push(hash_i);
                }

                tree.insert(insert_list);
            }

            for i in  ((16 / gap) * gap)..16 {
                let hash_i = poseidon(vec![&[i]]);
                let insert_list = vec![hash_i];
                tree.insert(insert_list);
            }

            gap += 1;
            assert_ne!(root, tree.root());
            assert_eq!(tree.next_leaf_index, 16);
            root_lists.push(tree.root());
        }

        for i in 0..root_lists.len() - 1 {
            assert_eq!(root_lists[i], root_lists[i + 1]);
        }
    }

    #[test]
    fn test_generate_proof() {
        const TREE_DEPTH: usize = 5;
        let mut tree = MerkleTreeSparse::<TREE_DEPTH>::new(0);

        let mut insert_list = vec![];
        for i in 0..8{
            let hash_i = poseidon(vec![&[i]]);
            insert_list.push(hash_i);
        }

        tree.insert(insert_list);

        let hash_5 = poseidon(vec![&[5]]);
        let mut path = Vec::with_capacity(5);
        path.push(poseidon(vec![&[4]]));
        path.push(hash_left_right(poseidon(vec![&[6]]), poseidon(vec![&[7]])));

        let hash_01 = hash_left_right(poseidon(vec![&[0]]), poseidon(vec![&[1]]));
        let hash_23 = hash_left_right(poseidon(vec![&[2]]), poseidon(vec![&[3]]));
        path.push(hash_left_right(hash_01, hash_23));
        path.push(tree.zeros[3].clone());

        let proof = tree.generate_proof(hash_5);

        assert_eq!(proof.index, 5);
        assert_eq!(path, proof.path);
    }

    #[test]
    fn test_proof_check() {
        const TREE_DEPTH: usize = 5;
        let mut tree = MerkleTreeSparse::<TREE_DEPTH>::new(0);

        let mut insert_list = vec![];
        for i in 0..8{
            let hash_i = poseidon(vec![&[i]]);
            insert_list.push(hash_i);
        }

        tree.insert(insert_list);

        let hash_5 = poseidon(vec![&[5]]);
        let proof = tree.generate_proof(hash_5.clone());

        merkle_proof_check(hash_5, proof.index, proof.root, proof.path, 5);
    }

    fn merkle_proof_check(
        leaf: Vec<u8>,
        leaf_index: u64,
        root: Vec<u8>,
        path: Vec<Vec<u8>>,
        tree_depth: u64,
    ) {
        let mut current_hash = leaf;
        let mut index = leaf_index;
    
        assert!(path.len() == (tree_depth-1) as usize);
    
        for sibling in path.iter() {
            let (left, right) = if index % 2 == 0 {
                (current_hash.clone(), sibling.clone())
            } else {
                (sibling.clone(), current_hash.clone())
            };
    
            current_hash = hash_left_right(left, right);
            index /= 2;
        }
    
        assert_eq!(current_hash, root);
    }
    
}
