use rs_merkle::{algorithms::Sha256, Hasher, MerkleTree};
use serde::{Deserialize, Serialize};

pub type RootHash = [u8; 32];

#[derive(Default)]
pub struct NodeState {
    pub trusted: RootHash,
    pub on_da: RootHash,
    pub on_da_finalized: RootHash,
}

pub trait StateMachine {
    fn dispatch(&mut self, key: u8, value: u64);
}

/// Custom leaves datastructure to allow serialization / deserialization.
///
/// This is created as a workaround since the library itself
/// does not expose this functionality.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Leaves {
    items: Vec<[u8; 32]>,
}

impl Leaves {
    pub fn set(&mut self, key: u8, value: [u8; 32]) {
        self.items[key as usize] = value;
    }

    #[allow(dead_code)]
    pub fn get(&self, key: u8) -> [u8; 32] {
        self.items[key as usize]
    }

    #[allow(dead_code)]
    pub fn inner(&self) -> &Vec<[u8; 32]> {
        &self.items
    }
}

pub struct State {
    pub leaves: Leaves,
    merkle_tree: MerkleTree<Sha256>,
}

impl State {
    pub fn new() -> Self {
        let values: [u64; 256] = [0; 256];
        let leaves: Vec<[u8; 32]> = values
            .iter()
            .map(|x| Sha256::hash(&x.to_be_bytes()))
            .collect();
        Self {
            leaves: Leaves {
                items: leaves.clone(),
            },
            merkle_tree: MerkleTree::from_leaves(&leaves),
        }
    }

    pub fn update(&mut self) {
        // This is somewhat a HACK since rs_merkle does not expose
        // an interface to modify the leaves or a merkle tree at a given position.
        // So we have to reconstruct the tree everytime.
        // This also prevents us from being able to use commit / rollback
        // functionalith this library provides.
        // Solution would be to fork the library to allow such
        // behavior (insert leaf at position).
        let mut updated_tree = MerkleTree::new();
        updated_tree.append(&mut self.leaves.items.clone());
        self.merkle_tree = updated_tree;
    }

    pub fn root(&self) -> Option<RootHash> {
        self.merkle_tree.uncommitted_root()
    }

    pub fn override_state(&mut self, leaves: Leaves) {
        self.leaves = leaves;
        self.update()
    }
}

impl StateMachine for State {
    fn dispatch(&mut self, key: u8, value: u64) {
        self.leaves.set(key, Sha256::hash(&value.to_be_bytes()));
        self.update();
    }
}
