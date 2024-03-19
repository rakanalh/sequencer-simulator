use std::collections::BTreeMap;

use rs_merkle::{algorithms::Sha256, Hasher, MerkleTree};
use serde::{Deserialize, Serialize};

pub type RootHash = [u8; 32];
pub type Leaves = BTreeMap<u8, Leaf>;

/// A trait for state-transitions
pub trait StateMachine {
    fn dispatch(&mut self, key: u8, value: u64);
}

/// The leaf's finalization status.
#[derive(Clone, Default, Serialize, Deserialize)]
pub enum FinalizationStatus {
    #[default]
    Trusted,
    DaNotFinalized,
    DaFinalized,
}

/// A custom structure to hold leaf's data and status.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Leaf {
    pub value: u64,
    pub status: FinalizationStatus,
}

#[derive(Default)]
pub struct Roots {
    pub trusted: RootHash,
    pub on_da: RootHash,
    pub on_da_finalized: RootHash,
}

pub struct State {
    // We use a BTreeMap for leaves since reading is efficient,
    // where search can cost O(log N).
    leaves: Leaves,
    merkle_tree: MerkleTree<Sha256>,
}

impl State {
    pub fn new() -> Self {
        let mut leaves = Leaves::new();
        for i in 0..256 {
            leaves.insert(i as u8, Default::default());
        }
        Self {
            leaves,
            merkle_tree: MerkleTree::from_leaves(&[[0; 32]; 256]),
        }
    }

    pub fn get(&self, key: u8) -> Leaf {
        self.leaves
            .get(&key)
            .unwrap_or(&Leaf {
                value: 0,
                status: FinalizationStatus::Trusted,
            })
            .clone()
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
        let mut leaves: Vec<[u8; 32]> = self
            .leaves
            .values()
            .map(|v| Sha256::hash(&v.value.to_be_bytes()))
            .collect();
        updated_tree.append(&mut leaves);
        self.merkle_tree = updated_tree;
    }

    pub fn root(&self) -> Option<RootHash> {
        self.merkle_tree.uncommitted_root()
    }

    pub fn override_leaves(&mut self, leaves: Leaves) {
        self.leaves = leaves;
        self.update()
    }

    pub fn leaves(&self) -> Leaves {
        self.leaves.clone()
    }
}

impl StateMachine for State {
    fn dispatch(&mut self, key: u8, value: u64) {
        self.leaves.insert(
            key,
            Leaf {
                value,
                status: FinalizationStatus::Trusted,
            },
        );
        self.update();
    }
}
