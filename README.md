Circuit and Client for veil program

Verfication program take the setup of the circuit and use them to verify using ```groth16_solana```.

## Verification circuit:
We utilize a Merkle tree to store UTXOs, where each leaf node represents a commitment. This commitment is the hash of the UTXO key, the amount, and the token information. The UTXO key itself is derived by hashing the master public key with random bytes.

When shielding transactions, the payer must generate two keys:

1. **Signing Key** – Used to sign transactions.
2. **Viewing Key** – Used to derive the nullifying key.

The nullifying key is the hash of the viewing key. The master public key is then derived by converting the signing key from private to public and hashing it together with the nullifying key.



The verificaion circuit contains of 3 components:

### 1.Verify merkle tree

Payer need to prove that they know where all the UTXOs they use to transfer located in merkle tree.
Public inputs:
- Path to Merkle leaf: root node, leaf node and sibling hashes at each level.
- Leaf index: The position of the UTXO in the tree.
    
Process: 
- Hash the leaf node with its corresponding sibling node to compute the next-level hash.
- Repeat the process iteratively until reaching the root node.
- Verify that the computed root hash matches the given root node.

Currently, the Merkle tree verification supports only **single-tree proofs**. Future improvements may include support for verifying UTXOs across multiple trees.

### 2. Nullifier check

Payer need to prove that they are the owner of the UTXO (which mean they own the secret key of that leaf's nulifier). That info will be store in program in order to prevent futher transaction on that leaf node.

Public inputs:
- Nullifying key (hash of viewing key)
- Leaf index
- Nullifier

The nullifier check only need to prove that the hash of nullifying key and leaf index equal to nullifier.

### 3. Signature & sum check

Payer need to sign a nullifier message to prove that they have the right to use that leaf node. As long as the **master public key** is stored in the leaf, the signing key must be correct.

Public inputs:
- Signing publicKey (Convert signing key from private to public)
- Signature
- Nulifier of inputs
- UXTO outputs
- Message hash
- Amount In/Out

Process:
- Construct message hash by combining: 
    - Merkle root
    - Bound params
    - Nulifier of inputs
    - UTXO outputs
- Verify message hash with signature
- Sum up the input amount, output amount. Check if sumIn == sumOut
