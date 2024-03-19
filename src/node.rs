use anyhow::{bail, Result};

use crate::state_machine::{FinalizationStatus, Leaves, Roots, State, StateMachine};

pub trait StateProvider {
    /// Returns the value at the given key, or 0 if the key is not present.
    /// Returns the finalization status of the value.
    #[allow(dead_code)]
    fn get(&self, key: u8) -> (u64, FinalizationStatus);
    /// Returns the value at block height for the given key,
    /// or 0 if the key is not present.
    #[allow(dead_code)]
    fn get_historical(&self, key: u8, block: u64) -> u64;
}

#[derive(Copy, Clone, Debug)]
pub enum StateType {
    Sequencer,
    DA,
}

// Node structure to maintaining state across sequencer and DA.
pub struct Node {
    // DB to store information on blocks.
    storage: sled::Db,
    // Sequencer state.
    pub sequencer_state: State,
    // Sequencer block number.
    pub sequencer_block_number: u64,
    // DA state.
    pub da_state: State,
    // DA block number.
    pub da_block_number: u64,
    // Maintain Merkle roots.
    pub roots: Roots,
}

impl Node {
    pub fn new(storage: sled::Db) -> Self {
        Self {
            storage,
            sequencer_state: State::new(),
            da_state: State::new(),
            roots: Default::default(),
            sequencer_block_number: 0,
            da_block_number: 0,
        }
    }

    // Submit state changes to either state machines based on state type.
    pub fn dispatch_state_change(&mut self, state_type: StateType, key: u8, value: u64) {
        match state_type {
            StateType::Sequencer => {
                self.sequencer_state.dispatch(key, value);
            }
            StateType::DA => self.da_state.dispatch(key, value),
        };
    }

    pub fn is_state_match(&mut self) -> bool {
        self.da_state.root() == self.sequencer_state.root()
    }

    // Revert based on DA blocks to revert.
    pub fn revert_blocks(
        &mut self,
        state_type: StateType,
        number_of_blocks_to_revert: u64,
    ) -> Result<()> {
        let current_block_number = self.current_block_number(state_type);
        let target_block = current_block_number - number_of_blocks_to_revert;

        // Restore leaves to DA state
        let leaves: Leaves = self.leaves_from_storage(state_type, target_block)?;

        match state_type {
            StateType::Sequencer => {
                self.sequencer_state.override_leaves(leaves);
                self.sequencer_block_number = target_block;
            }
            StateType::DA => {
                self.da_state.override_leaves(leaves);
                self.da_block_number = target_block;
            }
        };

        Ok(())
    }

    pub fn trust_block(&mut self) -> Result<()> {
        let Some(block_state_root) = self.sequencer_state.root() else {
            bail!("Could not compute sequencer state root");
        };

        self.sequencer_block_number += 1;

        // Set trusted Sequencer block
        self.roots.trusted = block_state_root;

        self.leaves_to_storage(StateType::Sequencer, self.sequencer_block_number)?;

        Ok(())
    }

    pub fn publish_block(&mut self) -> Result<()> {
        self.da_block_number += 1;
        self.roots.on_da = self.roots.trusted;
        self.leaves_to_storage(StateType::DA, self.da_block_number)?;
        Ok(())
    }

    pub fn finalize_block(&mut self) -> Result<()> {
        // TODO: Optimize for storage by clearning out blocks prior to the finalized
        // block, since we cannot revert a finalized block.
        let Some(block_state_root) = self.da_state.root() else {
            bail!("Could not compute DA state root");
        };
        self.roots.on_da_finalized = block_state_root;
        Ok(())
    }

    fn current_block_number(&self, state_type: StateType) -> u64 {
        match state_type {
            StateType::DA => self.da_block_number,
            StateType::Sequencer => self.sequencer_block_number,
        }
    }

    fn leaves_from_storage(&self, state_type: StateType, target_block: u64) -> Result<Leaves> {
        let key = match state_type {
            StateType::DA => "da-block",
            StateType::Sequencer => "seq-block",
        };
        let leaves_str: String = std::str::from_utf8(
            &self
                .storage
                .get(format!("{}-{}", key, target_block))?
                .expect("Should be set"),
        )?
        .into();
        Ok(serde_json::from_str(&leaves_str)?)
    }

    fn leaves_to_storage(&mut self, state_type: StateType, block_number: u64) -> Result<()> {
        let (key, leaves) = match state_type {
            StateType::DA => ("da", self.da_state.leaves()),
            StateType::Sequencer => ("seq", self.sequencer_state.leaves()),
        };

        let leaves = serde_json::to_string(&leaves)?;
        self.storage
            .insert(format!("{}-block-{}", key, block_number), leaves.as_str())?;
        self.storage.insert(
            format!("{}-current-block", key),
            block_number.to_string().as_str(),
        )?;

        Ok(())
    }
}

/// Provide state for Sequencer
impl StateProvider for Node {
    #[allow(dead_code)]
    fn get(&self, key: u8) -> (u64, FinalizationStatus) {
        let leaf = self.sequencer_state.get(key);
        (leaf.value, leaf.status)
    }

    fn get_historical(&self, key: u8, block: u64) -> u64 {
        let mut state = State::new();

        // Restore leaves to DA state
        if let Ok(leaves) = self.leaves_from_storage(StateType::Sequencer, block) {
            state.override_leaves(leaves);
            let leaf = state.get(key);
            leaf.value
        } else {
            0
        }
    }
}
