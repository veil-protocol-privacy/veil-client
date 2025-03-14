use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::U256;
use solana_poseidon::hashv;
use solana_poseidon::{Endianness, Parameters, PoseidonHash};
use std::collections::HashMap;

fn hash_left_right(left: Vec<u8>, right: Vec<u8>) -> Result<Vec<u8>, String> {
    let result: Result<PoseidonHash, solana_poseidon::PoseidonSyscallError> =
        hashv(Parameters::Bn254X5, Endianness::BigEndian, &[&left, &right]);

    match result {
        Ok(hash) => {
            let bytes = hash.to_bytes();
            return Ok(bytes.to_vec());
        }
        Err(err) => {
            return Err(format!("fail to create hash: {}", err.to_string()));
        }
    }
}

// Batch Incremental Merkle Tree for commitments
// each account store a single tree indicate by its
// tree number
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct CommitmentsAccount<const TREE_DEPTH: usize> {
    pub next_leaf_index: usize,
    merkle_root: Vec<u8>,
    new_tree_root: Vec<u8>,
    tree_number: u64,
    zeros: Vec<Vec<u8>>,
    filled_sub_trees: Vec<Vec<u8>>,
    root_history: HashMap<Vec<u8>, bool>, // root -> seen
}

// InsertResp return the tree number the insertion occur
// the leaf index, updated commitments data and the address
// that store the data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct InsertResp<const TREE_DEPTH: usize> {
    pub commitments_data: CommitmentsAccount<TREE_DEPTH>,
}

impl<const TREE_DEPTH: usize> CommitmentsAccount<TREE_DEPTH> {

    /// Create a new empty Merkle Tree
    pub fn new(tree_number: u64) -> Self {
        let zero_value = u256_to_bytes(ZERO_VALUE).to_vec();
        let mut root_history: HashMap<Vec<u8>, bool> = HashMap::new();
        let mut zeros: Vec<Vec<u8>> = Vec::with_capacity(TREE_DEPTH);
        let mut filled_sub_trees: Vec<Vec<u8>> = Vec::with_capacity(TREE_DEPTH);

        let mut current_zero = zero_value.clone();
        for _ in 0..TREE_DEPTH {
            // Push it to zeros array
            zeros.push(current_zero.clone());

            filled_sub_trees.push(current_zero.clone());

            // Calculate the zero value for this level
            current_zero = hash_left_right(current_zero.clone(), current_zero.clone()).unwrap();
        }

        // Now safely insert into the inner HashMap
        root_history.insert(current_zero.clone(), true);

        Self {
            next_leaf_index: 0,
            merkle_root: current_zero.clone(),
            new_tree_root: current_zero.clone(),
            tree_number,
            zeros,
            filled_sub_trees,
            root_history,
        }
    }

    /// Batch insert multiple commitments
    pub fn insert_commitments(
        &mut self,
        commitments: &mut Vec<Vec<u8>>,
    ) -> Result<InsertResp<TREE_DEPTH>, String> {
        // this check is just double check to make sure the leaf count does not exceed the limit
        // as above logic must also check this in order to create another data account
        // for a new tree if insertion exceeds the max tree dept.
        let mut count = commitments.len();

        if self.exceed_tree_depth(count) {
            return Err(format!("exceed max tree dept"));
        }

        let mut level_insertion_index: usize = self.next_leaf_index;

        self.next_leaf_index += count;

        // Variables for starting point at next tree level
        let mut next_level_hash_index: usize = 0;
        let mut next_level_start_index: usize;

        // Loop through each level of the merkle tree and update
        for level in 0..TREE_DEPTH {
            // Calculate the index to start at for the next level
            // >> is equivalent to / 2 rounded down
            next_level_start_index = level_insertion_index >> 1;

            let mut insertion_element = 0;

            // If we're on the right, hash and increment to get on the left
            if level_insertion_index % 2 == 1 {
                // Calculate index to insert hash into leafHashes[]
                // >> is equivalent to / 2 rounded down
                next_level_hash_index = (level_insertion_index >> 1) - next_level_start_index;

                // Calculate the hash for the next level
                commitments[next_level_hash_index] = hash_left_right(
                    self.filled_sub_trees[level].clone(),
                    commitments[insertion_element].clone(),
                )?;

                // Increment
                insertion_element += 1;
                level_insertion_index += 1;
            }

            // We'll always be on the left side now
            for insertion_element in (insertion_element..count).step_by(2) {
                let &mut right: &mut Vec<u8>;

                // Calculate right value
                if insertion_element < count - 1 {
                    right = commitments[insertion_element + 1].clone();
                } else {
                    right = self.zeros[level].clone();
                }

                // If we've created a new subtree at this level, update
                if insertion_element == count - 1 || insertion_element == count - 2 {
                    self.filled_sub_trees[level] = commitments[insertion_element].clone();
                }

                // Calculate index to insert hash into leafHashes[]
                // >> is equivalent to / 2 rounded down
                next_level_hash_index = (level_insertion_index >> 1) - next_level_start_index;

                // Calculate the hash for the next level
                commitments[next_level_hash_index] = hash_left_right(commitments[insertion_element].clone(), right)?;

                // Increment level insertion index
                level_insertion_index += 2;
            }

            // Get starting levelInsertionIndex value for next level
            level_insertion_index = next_level_start_index;

            // Get count of elements for next level
            count = next_level_hash_index + 1;
        }

        // Update the Merkle tree root
        self.merkle_root = commitments[0].clone();
        self
            .root_history
            .insert(self.merkle_root.clone(), true);

        Ok(InsertResp {
            commitments_data: self.clone(),
        })
    }

    pub fn exceed_tree_depth(&self, commitments_length: usize) -> bool {
        let base: usize = 2; // an explicit type is required
                             // if exceeding max tree depth create a new tree
        if commitments_length + self.next_leaf_index > base.pow(TREE_DEPTH as u32) {
            return true
        }

        return false;
    }

    /// Get the Merkle root
    pub fn root(&self) -> Vec<u8> {
        self.merkle_root.clone()
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

    fn poseidon(inputs: Vec<&[u8]>) -> Vec<u8> {
        hashv(Parameters::Bn254X5, Endianness::BigEndian, &inputs).unwrap().to_bytes().to_vec()
    }
    #[test]
    fn test_zero_tree() {
        let zero_value = u256_to_bytes(ZERO_VALUE).to_vec();
        const TREE_DEPTH: usize = 8;
        let zero_tree = CommitmentsAccount::<TREE_DEPTH>::new(0);
        let mut level_zero = zero_value.clone();
        for i in 0..TREE_DEPTH {
            assert_eq!(zero_tree.zeros[i], level_zero);
            assert_eq!(zero_tree.filled_sub_trees[i], level_zero);

            level_zero = hash_left_right(level_zero.clone(), level_zero.clone()).unwrap();
        }

        assert_eq!(zero_tree.merkle_root, level_zero);
        assert!(zero_tree.root_history.contains_key(&level_zero));
    }
    
    #[test]
    fn test_insert() {
        const TREE_DEPTH: usize = 5;

        let mut gap = 1;
        let mut root_lists = vec![];
        while gap < 10 {
            let mut tree = CommitmentsAccount::<TREE_DEPTH>::new(0);
            let root = tree.root();

            for step in 0..(16 / gap) {
                let mut insert_list = vec![];
                for i in (step * gap)..((step + 1) * gap) {
                    let hash_i = poseidon(vec![&[i]]);
                    insert_list.push(hash_i);
                }

                tree.insert_commitments(&mut insert_list).unwrap();
            }

            for i in  ((16 / gap) * gap)..16 {
                let hash_i = poseidon(vec![&[i]]);
                let mut insert_list = vec![hash_i];
                tree.insert_commitments(&mut insert_list).unwrap();
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
    fn test_exceed_tree() {
        const TREE_DEPTH: usize = 5;
        let mut tree = CommitmentsAccount::<TREE_DEPTH>::new(0);
        let mut insert_list = vec![];
        for i in 0..33 {
            let hash_i = poseidon(vec![&[i]]);
            insert_list.push(hash_i);
        }

        let result = tree.insert_commitments(&mut insert_list);
        assert!(result.is_err());
    }
}