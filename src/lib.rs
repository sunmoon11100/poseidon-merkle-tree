use std::sync::Mutex;

use ark_bn254::Fr;
use borsh::{BorshDeserialize, BorshSerialize};
use light_poseidon::{Poseidon, PoseidonBytesHasher, PoseidonParameters};
use once_cell::sync::Lazy;
use thiserror::Error;

use circom_t3::{ARK, MDS};

mod circom_t3;

pub const MAX_LEVELS: usize = 20;

// Static Poseidon hasher initialized lazily and protected by a Mutex for thread safety
static POSEIDON: Lazy<Mutex<Poseidon<Fr>>> = Lazy::new(|| {
    let params = PoseidonParameters {
        ark: Vec::from(ARK),
        mds: MDS.iter().map(|row| row.to_vec()).collect(),
        full_rounds: 8,
        partial_rounds: 57,
        width: 3,
        alpha: 5,
    };
    Mutex::new(Poseidon::<Fr>::new(params))
});

#[derive(Error, Debug, PartialEq)]
pub enum PoseidonMerkleTreeError {
    #[error("Invalid levels")]
    InvalidLevels,

    #[error("Merkle tree is full")]
    MerkleTreeFull,

    #[error("Failed to acquire Poseidon hasher lock")]
    PoseidonLockError,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct PoseidonMerkleTree {
    pub levels: u32,
    pub filled_subtrees: Vec<[u8; 32]>,
    pub roots: Vec<[u8; 32]>,
    pub current_root_index: u32,
    pub next_index: u32,
}

impl PoseidonMerkleTree {
    pub const SIZE: usize = 4 + 32 * MAX_LEVELS + 32 * MAX_LEVELS + 4 + 4;

    pub fn new(levels: u32) -> Result<PoseidonMerkleTree, PoseidonMerkleTreeError> {
        if levels > MAX_LEVELS as u32 {
            return Err(PoseidonMerkleTreeError::InvalidLevels);
        }

        // Initialize filled_subtrees with the correct zero values
        let filled_subtrees: Vec<[u8; 32]> = (0..levels).map(zeros).collect();

        // Initialize roots with zeros and set the first root
        let mut roots = [[0; 32]; MAX_LEVELS];
        roots[0] = zeros(levels - 1);

        Ok(PoseidonMerkleTree {
            levels,
            filled_subtrees,
            roots: roots.to_vec(),
            current_root_index: 0,
            next_index: 0,
        })
    }

    pub fn insert(&mut self, leaf: &[u8; 32]) -> Result<u32, PoseidonMerkleTreeError> {
        if self.next_index == 2u32.pow(self.levels) {
            return Err(PoseidonMerkleTreeError::MerkleTreeFull);
        }

        let mut current_index = self.next_index;
        let mut current_level_hash = leaf.clone();

        // Acquire the Poseidon hasher lock
        let mut poseidon = POSEIDON
            .lock()
            .map_err(|_| PoseidonMerkleTreeError::PoseidonLockError)?;

        for i in 0..self.levels {
            let (left, right) = if current_index % 2 == 0 {
                (current_level_hash, zeros(i))
            } else {
                (self.filled_subtrees[i as usize], current_level_hash)
            };

            current_level_hash = poseidon.hash_bytes_be(&[&left, &right]).unwrap();
            self.filled_subtrees[i as usize] = left;
            current_index /= 2;
        }

        let new_root_index = (self.current_root_index + 1) % MAX_LEVELS as u32;
        self.current_root_index = new_root_index;
        self.roots[new_root_index as usize] = current_level_hash;
        self.next_index += 1;

        Ok(self.next_index)
    }

    pub fn is_known_root(&self, root: [u8; 32]) -> bool {
        if root == [0; 32] {
            return false;
        }

        let mut i = self.current_root_index;
        for _ in 0..MAX_LEVELS {
            if self.roots[i as usize] == root {
                return true;
            }
            i = if i == 0 { MAX_LEVELS as u32 - 1 } else { i - 1 };
        }

        false
    }
}

fn zeros(i: u32) -> [u8; 32] {
    match i {
        0 => [
            0x28, 0x94, 0x0d, 0xee, 0xac, 0xd1, 0xca, 0x28, 0x31, 0x33, 0x68, 0x74, 0xe8, 0x74,
            0x29, 0xdb, 0x0e, 0x72, 0x8a, 0x67, 0xa4, 0x72, 0xb7, 0xac, 0x81, 0x95, 0xc4, 0x3c,
            0x2f, 0xb1, 0x30, 0x09,
        ], // sha("voidify")
        1 => [
            0x13, 0x8b, 0xfd, 0xb7, 0x91, 0xd8, 0xba, 0xd9, 0x8a, 0x50, 0xc8, 0x2e, 0xa1, 0xef,
            0x62, 0x4f, 0xeb, 0x03, 0xed, 0x9b, 0x7b, 0xbd, 0xb3, 0x48, 0x55, 0x1a, 0x6b, 0x34,
            0x7f, 0xfd, 0x56, 0x1c,
        ],
        2 => [
            0x00, 0x5e, 0xf3, 0xbb, 0xa3, 0x6e, 0x2d, 0x71, 0x45, 0x75, 0xef, 0x75, 0xc6, 0xec,
            0x27, 0xc6, 0x0e, 0x05, 0x93, 0xfb, 0x7b, 0xd4, 0x01, 0x2a, 0x33, 0x0b, 0xc0, 0x65,
            0xfb, 0x79, 0x08, 0x37,
        ],
        3 => [
            0x10, 0xc7, 0x03, 0x6d, 0x8a, 0x63, 0xd1, 0x40, 0xd7, 0x7c, 0x6a, 0xc1, 0x21, 0xc2,
            0xef, 0x50, 0x2c, 0xa8, 0x37, 0x03, 0x91, 0x3d, 0x34, 0x97, 0x48, 0x17, 0x54, 0x31,
            0x1c, 0xf8, 0x12, 0xa1,
        ],
        4 => [
            0x1e, 0x54, 0xdf, 0x31, 0x58, 0xcf, 0x89, 0x80, 0x2f, 0x13, 0xf7, 0x22, 0x65, 0xf2,
            0x6c, 0x3f, 0x28, 0x13, 0x91, 0x46, 0x57, 0xcc, 0xe8, 0xfe, 0x1c, 0x68, 0xc8, 0x1c,
            0x6f, 0x84, 0xb5, 0xe3,
        ],
        5 => [
            0x07, 0xf8, 0x79, 0x07, 0xf4, 0x8e, 0x61, 0x7a, 0x18, 0x4d, 0x93, 0x59, 0x64, 0x50,
            0xb3, 0xa6, 0x8a, 0x30, 0xc0, 0xdf, 0xdf, 0x93, 0x16, 0x4a, 0x0a, 0xf9, 0x63, 0xdd,
            0xcc, 0xc0, 0x4c, 0xc7,
        ],
        6 => [
            0x1b, 0xca, 0xbd, 0x63, 0x5e, 0x6f, 0x84, 0x5b, 0x50, 0x39, 0xcb, 0xf8, 0x27, 0xb5,
            0x28, 0x12, 0x1e, 0xc3, 0x4a, 0x2a, 0x3f, 0x68, 0x0f, 0x27, 0xf8, 0x84, 0x56, 0xc4,
            0x76, 0x62, 0xec, 0x32,
        ],
        7 => [
            0x03, 0x2d, 0x93, 0x0e, 0x15, 0x6c, 0xce, 0x79, 0x7f, 0xcd, 0x3f, 0x4a, 0x11, 0xdc,
            0x41, 0x70, 0x31, 0x5f, 0x8f, 0x83, 0x0c, 0xa6, 0xb0, 0xf3, 0xbb, 0x71, 0x1e, 0x53,
            0x37, 0xd6, 0x77, 0x3d,
        ],
        8 => [
            0x17, 0x0a, 0xbe, 0x49, 0x47, 0xc1, 0x19, 0x5a, 0x40, 0xa4, 0x88, 0x11, 0xe6, 0xb3,
            0x62, 0xa0, 0xa9, 0xc8, 0x68, 0x57, 0x33, 0xc1, 0x7f, 0x61, 0x50, 0xc1, 0x96, 0xb9,
            0x39, 0xfc, 0x21, 0xf8,
        ],
        9 => [
            0x03, 0xd9, 0xe6, 0x48, 0xd6, 0x74, 0x27, 0xd0, 0xa6, 0xe0, 0xa3, 0x0a, 0xad, 0x5d,
            0x18, 0xaf, 0x05, 0xb9, 0xe0, 0x4b, 0x41, 0xb4, 0x98, 0x5f, 0xd4, 0x06, 0x2d, 0xe2,
            0x71, 0x1c, 0xbe, 0xc1,
        ],
        10 => [
            0x04, 0xa4, 0xfe, 0x12, 0x21, 0xc0, 0xd2, 0x1b, 0x27, 0xb4, 0x9a, 0x23, 0xb7, 0x53,
            0x47, 0xfe, 0xc6, 0x90, 0x3b, 0xba, 0xd2, 0xf6, 0x12, 0x99, 0xb9, 0x36, 0xbf, 0xb7,
            0xb7, 0x83, 0xfc, 0xd7,
        ],
        11 => [
            0x14, 0x32, 0xaa, 0x33, 0x5f, 0xcc, 0xae, 0xed, 0xed, 0x95, 0x05, 0xa5, 0xa1, 0x42,
            0xe8, 0x56, 0x8a, 0xf6, 0x2c, 0xcc, 0x90, 0x81, 0x14, 0xbf, 0xdc, 0xbe, 0x95, 0x6e,
            0x11, 0x72, 0xad, 0x98,
        ],
        12 => [
            0x18, 0x91, 0x90, 0x59, 0xfd, 0x2a, 0x3d, 0x7b, 0xa6, 0xc4, 0x04, 0x9f, 0x42, 0xb7,
            0x7b, 0x0e, 0xcc, 0x6a, 0x23, 0x01, 0xe6, 0x65, 0x36, 0x38, 0x7f, 0x11, 0xaa, 0x52,
            0x2b, 0x3e, 0xd2, 0x7b,
        ],
        13 => [
            0x06, 0x96, 0x2f, 0x22, 0x9c, 0x6f, 0x6e, 0x30, 0x7a, 0x60, 0x22, 0x49, 0x33, 0xcb,
            0x0d, 0x9c, 0x9b, 0x61, 0xcf, 0x44, 0x2e, 0xd5, 0xb0, 0x36, 0xe9, 0xcf, 0x36, 0x70,
            0xa5, 0xaf, 0xf8, 0xd2,
        ],
        14 => [
            0x01, 0x82, 0x1e, 0x95, 0xe5, 0x34, 0x93, 0x44, 0x8e, 0x2d, 0x59, 0x9c, 0xb0, 0x45,
            0xcd, 0x8e, 0x8d, 0x21, 0xf3, 0xd2, 0xd7, 0xe8, 0xac, 0xf5, 0xc9, 0x09, 0x68, 0x1e,
            0xe2, 0x0a, 0x69, 0x26,
        ],
        15 => [
            0x0e, 0xc5, 0xb2, 0x9a, 0xd4, 0x60, 0x9e, 0xfd, 0x69, 0xbd, 0x92, 0x30, 0xc8, 0x9f,
            0x82, 0xf3, 0xfc, 0x15, 0x03, 0xf3, 0x8c, 0x21, 0x15, 0x07, 0x3e, 0x82, 0x22, 0x61,
            0x91, 0x92, 0x62, 0x96,
        ],
        16 => [
            0x16, 0x4c, 0x52, 0x2e, 0xc8, 0xd8, 0xd0, 0x64, 0xe9, 0xac, 0x53, 0x5c, 0x6a, 0x1b,
            0x34, 0xfc, 0x41, 0xa5, 0x05, 0xd8, 0x70, 0xeb, 0xc0, 0xad, 0x55, 0x16, 0x72, 0x17,
            0x1b, 0x75, 0xf3, 0x4c,
        ],
        17 => [
            0x25, 0x2a, 0x2a, 0xcf, 0xa2, 0x2c, 0xa0, 0x9d, 0x7f, 0x96, 0x5d, 0x01, 0x5b, 0x01,
            0xcf, 0x3c, 0xd5, 0x9f, 0xf8, 0x9d, 0x5b, 0x4f, 0x22, 0x95, 0x64, 0xc2, 0x28, 0xf2,
            0x50, 0x20, 0xed, 0xf1,
        ],
        18 => [
            0x2f, 0x72, 0x9a, 0xb9, 0x99, 0x4d, 0x06, 0xf1, 0xe6, 0xc0, 0x77, 0xc5, 0xea, 0xdb,
            0xc4, 0x51, 0xe7, 0x21, 0xd0, 0x29, 0x15, 0x9a, 0x30, 0xe4, 0x7e, 0x32, 0xb1, 0x5c,
            0xc6, 0xe2, 0x8a, 0xb7,
        ],
        19 => [
            0x19, 0xbf, 0x0a, 0x91, 0xf2, 0x85, 0x2d, 0x3a, 0x5b, 0xd3, 0x56, 0x5d, 0x9f, 0x77,
            0xe0, 0x4f, 0xb6, 0xde, 0x7b, 0xc3, 0x18, 0x75, 0x3f, 0xa5, 0x28, 0x17, 0x00, 0xd7,
            0x86, 0xe8, 0xab, 0xd1,
        ],
        20 => [
            0x28, 0xc6, 0xd1, 0x55, 0xc4, 0xef, 0x4f, 0x87, 0x09, 0x53, 0x23, 0xe8, 0x83, 0x2e,
            0xc0, 0x54, 0xfa, 0x7d, 0xab, 0x72, 0xa6, 0xfd, 0x22, 0x95, 0x6b, 0x39, 0xe3, 0xdb,
            0x18, 0x40, 0x29, 0x6f,
        ],
        _ => panic!("Index out of bounds"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_levels() {
        let tree = PoseidonMerkleTree::new(5).unwrap();
        assert_eq!(tree.levels, 5);
        assert_eq!(tree.filled_subtrees.len(), 5);
        assert_eq!(tree.roots.len(), MAX_LEVELS);
        assert_eq!(tree.current_root_index, 0);
        assert_eq!(tree.next_index, 0);
        assert_eq!(tree.roots[0], zeros(4)); // Root should be zero hash for level 4
        assert_eq!(tree.filled_subtrees[0], zeros(0));
        assert_eq!(tree.filled_subtrees[4], zeros(4));
    }

    #[test]
    fn test_new_invalid_levels() {
        let result = PoseidonMerkleTree::new(MAX_LEVELS as u32 + 1);
        assert_eq!(result, Err(PoseidonMerkleTreeError::InvalidLevels));
    }

    #[test]
    fn test_insert_single_leaf() {
        let mut tree = PoseidonMerkleTree::new(3).unwrap();
        let leaf = [1u8; 32];
        let result = tree.insert(&leaf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert_eq!(tree.next_index, 1);
        assert_eq!(tree.current_root_index, 1);
        assert_eq!(tree.filled_subtrees[0], leaf);
        assert_ne!(tree.roots[1], [0; 32]);
    }

    #[test]
    fn test_insert_multiple_leaves() {
        let mut tree = PoseidonMerkleTree::new(3).unwrap();
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];

        tree.insert(&leaf1).unwrap();
        tree.insert(&leaf2).unwrap();

        assert_eq!(tree.next_index, 2);
        assert_eq!(tree.current_root_index, 2);

        let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
        let expected_hash = poseidon.hash_bytes_be(&[&leaf1, &leaf2]).unwrap();
        assert_eq!(tree.filled_subtrees[1], expected_hash);
    }

    #[test]
    fn test_merkle_tree_full() {
        let mut tree = PoseidonMerkleTree::new(2).unwrap();
        let leaf = [1u8; 32];

        // Insert 4 leaves (2^2 = 4)
        tree.insert(&leaf).unwrap();
        tree.insert(&leaf).unwrap();
        tree.insert(&leaf).unwrap();
        tree.insert(&leaf).unwrap();

        // Next insert should fail
        let result = tree.insert(&leaf);
        assert_eq!(result, Err(PoseidonMerkleTreeError::MerkleTreeFull));
    }

    #[test]
    fn test_is_known_root() {
        let mut tree = PoseidonMerkleTree::new(3).unwrap();
        let leaf = [1u8; 32];
        tree.insert(&leaf).unwrap();

        let current_root = tree.roots[1];
        assert!(tree.is_known_root(current_root));
        assert!(!tree.is_known_root([0; 32]));
        assert!(!tree.is_known_root([2u8; 32]));
    }

    #[test]
    fn test_root_history() {
        let mut tree = PoseidonMerkleTree::new(3).unwrap();
        let leaf = [1u8; 32];

        // Insert enough leaves to fill some root history
        for i in 0..5 {
            tree.insert(&leaf).unwrap();
            let root = tree.roots[(i + 1) as usize];
            assert!(tree.is_known_root(root));
        }

        // Check that older roots are still recognized
        let first_root = tree.roots[1];
        assert!(tree.is_known_root(first_root));
    }
}
