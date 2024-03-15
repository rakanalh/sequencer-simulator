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

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Leaves {
    items: Vec<[u8; 32]>,
}

impl Leaves {
    pub fn set(&mut self, key: u8, value: [u8; 32]) {
        self.items[key as usize] = value;
    }

    pub fn get(&self, key: u8) -> [u8; 32] {
        self.items[key as usize]
    }

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
        let mut updated_tree = MerkleTree::new();
        updated_tree.append(&mut self.leaves.items.clone());
        self.merkle_tree = updated_tree;
    }

    pub fn commit(&mut self) {}

    pub fn root(&self) -> Option<RootHash> {
        self.merkle_tree.uncommitted_root()
    }

    pub fn override_state(&mut self, leaves: Leaves) {
        self.leaves = leaves
    }
}

impl StateMachine for State {
    fn dispatch(&mut self, key: u8, value: u64) {
        // println!(
        //     "Key: {}\nValue: {}\nBytes {:?}",
        //     key,
        //     value,
        //     Sha256::hash(&value.to_be_bytes())
        // );
        self.leaves.set(key, Sha256::hash(&value.to_be_bytes()));
        self.update();
    }
}
