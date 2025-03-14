use rand::Rng;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use merkle::CommitmentsAccount;

pub mod merkle;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const METHOD_ELF: &[u8] = include_elf!("methods");

fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..length).map(|_| rng.random()).collect()
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    let signing_key = generate_random_bytes(32);
    let viewing_key = generate_random_bytes(32);
    let tree: CommitmentsAccount<16> = CommitmentsAccount::new(0);

    // Add some money to merkle tree

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();

    // TODO: update inputs
    stdin.write(&1);


    // Setup the program for proving.
    let (pk, vk) = client.setup(METHOD_ELF);

    // Generate the proof
    let proof = client
        .prove(&pk, &stdin)
        .groth16()
        .run()
        .expect("failed to generate proof");

    println!("Successfully generated proof!");

    // TODO: decypt vkey for program compatible
    
    // Verify the proof.
    client.verify(&proof, &vk).expect("failed to verify proof");
    println!("Successfully verified proof!");
}
