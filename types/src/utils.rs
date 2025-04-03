use sha3::{Digest, Sha3_256};

pub fn poseidon(inputs: Vec<&[u8]>) -> Vec<u8> {
    let inputs = inputs
        .iter()
        .map(|input| {
            let mut bytes = [0u8; 32];
            if input.len() < 32 {
                // fill from the last index
                let start = 32 - input.len();
                bytes[start..].copy_from_slice(&input[..]);
            } else {
                bytes.copy_from_slice(input);
            };
            bytes
        })
        .collect::<Vec<[u8; 32]>>();
    Vec::from(
        solana_poseidon::hashv(
            solana_poseidon::Parameters::Bn254X5,
            solana_poseidon::Endianness::BigEndian,
            &inputs.iter().map(|v| v.as_slice()).collect::<Vec<&[u8]>>(),
        )
        .unwrap()
        .to_bytes(),
    )
}

pub fn hash_left_right(left: Vec<u8>, right: Vec<u8>) -> Vec<u8> {
    let mut left_bytes = [0u8; 32];
    left_bytes.copy_from_slice(&left);

    let mut right_bytes = [0u8; 32];
    right_bytes.copy_from_slice(&right);

    Vec::from(
        solana_poseidon::hashv(
            solana_poseidon::Parameters::Bn254X5,
            solana_poseidon::Endianness::BigEndian,
            &[&left_bytes, &right_bytes],
        )
        .unwrap()
        .to_bytes(),
    )
}

pub fn keccak(inputs: Vec<&[u8]>) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    for input in inputs {
        hasher.update(input);
    }

    let result = hasher.finalize();
    result.as_slice().to_vec()
}

pub fn generate_nullifier(viewing_key: Vec<u8>, leaf_index: u64) -> Vec<u8> {
    let nullifying_key = poseidon(vec![viewing_key.as_slice()]);
    let leaf_index_bytes = leaf_index.to_le_bytes().to_vec();
    poseidon(vec![nullifying_key.as_slice(), leaf_index_bytes.as_slice()])
}
