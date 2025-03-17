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