use starknet_crypto::{Felt, poseidon_hash_many};
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