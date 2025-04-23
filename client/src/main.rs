use client::merkle::{MerkleProof, MerkleTreeSparse};
use rand::Rng;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use types::{keccak, sha256, utxo::UTXO, Arguments, CipherText, PrivateData, PublicData};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const METHOD_ELF: &[u8] = include_elf!("methods");

fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng: rand::prelude::ThreadRng = rand::rng();
    (0..length).map(|_| rng.random()).collect()
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    let spending_key_1 = generate_random_bytes(32);
    let spending_key_2: Vec<u8> = generate_random_bytes(32);
    let viewing_key_1 = generate_random_bytes(32);
    let viewing_key_2 = generate_random_bytes(32);

    let random_1 = generate_random_bytes(32);
    let random_2 = generate_random_bytes(32);
    let random_3 = generate_random_bytes(32);

    let token_id = generate_random_bytes(32);
    let nonce = generate_random_bytes(12);

    let mut tree: MerkleTreeSparse<32> = MerkleTreeSparse::new(0);

    // Add some money to merkle tree
    let utxos_in = vec![
        UTXO::new(
            spending_key_1.clone(),
            viewing_key_1.clone(),
            token_id.clone(),
            random_1.clone(),
            nonce.clone(),
            200,
            "UTXO 1".to_string(),
        ),
        UTXO::new(
            spending_key_1.clone(),
            viewing_key_1.clone(),
            token_id.clone(),
            random_2.clone(),
            nonce.clone(),
            200,
            "UTXO 2".to_string(),
        ),
        UTXO::new(
            spending_key_1.clone(),
            viewing_key_1.clone(),
            token_id.clone(),
            random_3.clone(),
            nonce.clone(),
            200,
            "UTXO 3".to_string(),
        ),
    ];

    let utxos_out = vec![
        UTXO::new(
            spending_key_1.clone(),
            viewing_key_1.clone(),
            token_id.clone(),
            generate_random_bytes(32),
            nonce.clone(),
            300,
            "UTXO 4".to_string(),
        ),
        UTXO::new(
            spending_key_2.clone(),
            viewing_key_2.clone(),
            token_id.clone(),
            generate_random_bytes(32),
            nonce.clone(),
            300,
            "UTXO 5".to_string(),
        ),
    ];

    let commitments: Vec<Vec<u8>> = utxos_in.iter().map(|utxo| utxo.utxo_hash()).collect();
    tree.insert(commitments.clone());

    let mut fake_commitments = vec![];
    for i in 0..8 {
        let hash_i = sha256(vec![&[i]]);
        fake_commitments.push(hash_i);
    }
    tree.insert(fake_commitments);

    // TODO: hash params
    let merkle_root = tree.root();
    let params_hash = keccak(vec![&[100]]);

    let merkle_paths: Vec<Vec<Vec<u8>>> = commitments
        .iter()
        .map(|commitment| tree.generate_proof(commitment.clone()).path)
        .collect();

    let merkle_leaf_indices: Vec<u64> = commitments
        .iter()
        .map(|commitment| tree.generate_proof(commitment.clone()).index as u64)
        .collect();

    let nullifiers: Vec<Vec<u8>> = utxos_in
        .iter()
        .enumerate()
        .map(|(i, utxo_in)| utxo_in.nullifier(merkle_leaf_indices[i]))
        .collect();

    let output_hashes: Vec<Vec<u8>> = utxos_out.iter().map(|utxo| utxo.utxo_hash()).collect();

    let ciphertexts: Vec<CipherText> = utxos_in
        .iter()
        .map(|utxo| utxo.clone().encrypt(viewing_key_1.clone()))
        .collect();

    let pubkey = utxos_in[0].spending_public_key();
    let nullifying_key = utxos_in[0].nullifying_key();
    let signature = utxos_in[0].sign(
        merkle_root.clone(),
        params_hash.clone(),
        nullifiers.clone(),
        output_hashes.clone(),
    );
    let random_inputs = vec![random_1, random_2, random_3];
    let amount_in: Vec<u64> = vec![200, 200, 200];
    let amount_out: Vec<u64> = vec![300, 300];
    let utxo_output_keys: Vec<Vec<u8>> = utxos_out
        .iter()
        .map(|utxo| utxo.utxo_public_key())
        .collect();

    let public_data = PublicData {
        merkle_root,
        params_hash,
        nullifiers,
        output_hashes,
    };

    let private_data = PrivateData {
        token_id,
        pubkey,
        signature,
        random_inputs,
        amount_in,
        merkle_paths,
        merkle_leaf_indices,
        nullifying_key,
        utxo_output_keys,
        amount_out,
    };

    let args = Arguments {
        public_data,
        private_data,
        tree_depth: 32u64,
        input_count: 3u64,
        output_count: 2u64,
    };
    let serialized_args = borsh::to_vec(&args).unwrap();

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();

    stdin.write_vec(serialized_args);

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
