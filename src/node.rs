use anyhow::{bail, Result};

use crate::state_machine::{Leaves, NodeState, State, StateMachine};

pub struct Node {
    storage: sled::Db,
    sequencer_state: State,
    da_state: State,
    node_state: NodeState,
}

impl Node {
    pub fn new(storage: sled::Db) -> Self {
        Self {
            storage,
            sequencer_state: State::new(),
            da_state: State::new(),
            node_state: Default::default(),
        }
    }

    pub fn dispatch_sequencer_state_change(&mut self, key: u8, value: u64) {
        self.sequencer_state.dispatch(key, value);
    }

    pub fn publish_sequencer_block(&mut self, block_number: u64) -> Result<()> {
        let Some(block_state_root) = self.sequencer_state.root() else {
            bail!("Could not compute sequencer state root");
        };
        self.node_state.trusted = block_state_root;

        let leaves = serde_json::to_string(&self.sequencer_state.leaves)?;
        self.storage
            .insert(format!("seq-block-{}", block_number), leaves.as_str())?;
        self.storage
            .insert("seq-current-block", block_number.to_string().as_str())?;
        Ok(())
    }

    pub fn revert_seq_blocks(&mut self, number_of_blocks_to_revert: u64) -> Result<()> {
        let block_number_str: String = std::str::from_utf8(
            &self
                .storage
                .get("seq-current-block")?
                .expect("Should be set"),
        )?
        .into();

        let current_seq_block_number = block_number_str.parse::<u64>()?;
        let target_block = current_seq_block_number - number_of_blocks_to_revert;

        // Restore leaves to DA state
        let leaves_str: String = std::str::from_utf8(
            &self
                .storage
                .get(format!("seq-block-{}", target_block))?
                .expect("Should be set"),
        )?
        .into();
        let leaves: Leaves = serde_json::from_str(&leaves_str)?;
        self.sequencer_state.override_state(leaves);

        Ok(())
    }

    pub fn dispatch_da_state_change(&mut self, key: u8, value: u64) {
        self.da_state.dispatch(key, value)
    }

    pub fn ensure_state_match(&mut self) {
        assert_eq!(self.da_state.root(), self.sequencer_state.root());
    }

    pub fn update_da_block(&mut self, block_number: u64) -> Result<()> {
        self.node_state.on_da = self.node_state.trusted;
        let leaves = serde_json::to_string(&self.da_state.leaves)?;
        self.storage
            .insert(format!("da-block-{}", block_number), leaves.as_str())?;
        self.storage
            .insert("da-current-block", block_number.to_string().as_str())?;
        Ok(())
    }

    pub fn revert_da_blocks(&mut self, number_of_blocks_to_revert: u64) -> Result<()> {
        let block_number_str: String = std::str::from_utf8(
            &self
                .storage
                .get("da-current-block")?
                .expect("Should be set"),
        )?
        .into();

        let current_da_block_number = block_number_str.parse::<u64>()?;
        let target_block = current_da_block_number - number_of_blocks_to_revert;

        // Restore leaves to DA state
        let leaves_str: String = std::str::from_utf8(
            &self
                .storage
                .get(format!("da-block-{}", target_block))?
                .expect("Should be set"),
        )?
        .into();
        let leaves: Leaves = serde_json::from_str(&leaves_str)?;
        self.da_state.override_state(leaves);

        Ok(())
    }

    pub fn finalize_da_block(&mut self) {}
}
