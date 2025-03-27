use starknet_crypto::{Felt, poseidon_hash, poseidon_hash_many};
use sha3::{Digest, Sha3_256};

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


pub fn hash_left_right(left: Vec<u8>, right: Vec<u8>) -> Vec<u8> {
    let mut left_bytes = [0u8; 32];
    left_bytes.copy_from_slice(&left);

    let mut right_bytes = [0u8; 32];
    right_bytes.copy_from_slice(&right);

    Vec::from(poseidon_hash(Felt::from_bytes_be(&left_bytes), Felt::from_bytes_be(&right_bytes)).to_bytes_be())
}

pub fn keccak(
    inputs: Vec<&[u8]>
) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    for input in inputs {
        hasher.update(input);
    };

    let result = hasher.finalize();
    result.as_slice().to_vec()
}

pub fn generate_nullifier(
    viewing_key: Vec<u8>,
    leaf_index: u64,
) -> Vec<u8> {
    let nullifying_key = poseidon(vec![viewing_key.as_slice()]);
    let leaf_index_bytes = leaf_index.to_le_bytes().to_vec();
    poseidon(vec![nullifying_key.as_slice(), leaf_index_bytes.as_slice()])
}