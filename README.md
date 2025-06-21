# Poseidon Merkle Tree

This project implements a Poseidon-based Merkle Tree in Rust. It uses the Poseidon hash function for cryptographic operations, making it suitable for privacy-preserving applications.

## Features

- Initialize a Merkle Tree with a specified number of levels.
- Insert leaves into the Merkle Tree.
- Check if a root is known within the tree's history.
- Handle errors such as invalid levels and full trees.

```rust
use poseidon_merkle_tree::PoseidonMerkleTree;

fn main() {
    let mut tree = PoseidonMerkleTree::new(3).unwrap();
    let leaf = [1u8; 32];
    tree.insert(&leaf).unwrap();
    //assert!(tree.is_known_root(current_root));
    println!("Merkle Tree initialized and leaf inserted.");
}
```