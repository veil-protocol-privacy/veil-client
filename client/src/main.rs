use rand::Rng;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use merkle::MerkleTreeSparse;
use types::{utxo::UTXO, CipherText};

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

    let spending_key_1 = generate_random_bytes(32);
    let spending_key_2 = generate_random_bytes(32);
    let viewing_key_1 = generate_random_bytes(32);
    let viewing_key_2 = generate_random_bytes(32);

    let token_id = generate_random_bytes(32);
    let random_1 = generate_random_bytes(32);
    let random_2 = generate_random_bytes(32);
    let random_3 = generate_random_bytes(32);

    let nonce = generate_random_bytes(12);
    println!("nonce: {:?}", nonce);

    let mut tree: MerkleTreeSparse<32> = MerkleTreeSparse::new(0);

    // Add some money to merkle tree
    let utxos_in = vec![
        UTXO::new(spending_key_1.clone(), viewing_key_1.clone(), token_id.clone(), random_1.clone(), nonce.clone(), 200, "UTXO 1".to_string()),
        UTXO::new(spending_key_1.clone(), viewing_key_1.clone(), token_id.clone(), random_2.clone(), nonce.clone(), 200, "UTXO 2".to_string()),
        UTXO::new(spending_key_1.clone(), viewing_key_1.clone(), token_id.clone(), random_3.clone(), nonce.clone(), 200, "UTXO 3".to_string()),
    ];

    let utxos_out = vec![
        UTXO::new(spending_key_1.clone(), viewing_key_1.clone(), token_id.clone(), generate_random_bytes(32), nonce.clone(), 300, "UTXO 4".to_string()),
        UTXO::new(spending_key_2.clone(), viewing_key_2.clone(), token_id.clone(), generate_random_bytes(32), nonce.clone(), 300, "UTXO 5".to_string()),
    ];

    let commitments: Vec<Vec<u8>> = utxos_in.iter().map(|utxo| utxo.utxo_hash()).collect();
    tree.insert(commitments);

    let ciphertexts: Vec<CipherText> = utxos_in.iter().map(|utxo| utxo.clone().encrypt(viewing_key_1.clone())).collect();
    
    // // Setup the prover client.
    // let client = ProverClient::from_env();

    // // Setup the inputs.
    // let mut stdin = SP1Stdin::new();

    // // TODO: update inputs
    // stdin.write(&1);


    // // Setup the program for proving.
    // let (pk, vk) = client.setup(METHOD_ELF);

    // // Generate the proof
    // let proof = client
    //     .prove(&pk, &stdin)
    //     .groth16()
    //     .run()
    //     .expect("failed to generate proof");

    // println!("Successfully generated proof!");

    // // TODO: decypt vkey for program compatible
    
    // // Verify the proof.
    // client.verify(&proof, &vk).expect("failed to verify proof");
    // println!("Successfully verified proof!");
}
