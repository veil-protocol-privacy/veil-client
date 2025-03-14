use solana_poseidon::{hashv, Endianness, Parameters};

pub fn poseidon(
    inputs: Vec<&[u8]>
) -> Vec<u8> {
    Vec::from(hashv(Parameters::Bn254X5, Endianness::BigEndian, inputs.iter().as_slice()).unwrap().to_bytes())
}