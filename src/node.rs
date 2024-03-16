use anyhow::{bail, Result};

use crate::state_machine::{Leaves, NodeState, State, StateMachine};

#[derive(Copy, Clone)]
pub enum StateType {
    Sequencer,
    DA,
}

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

    pub fn dispatch_state_change(&mut self, state_type: StateType, key: u8, value: u64) {
        match state_type {
            StateType::Sequencer => self.sequencer_state.dispatch(key, value),
            StateType::DA => self.da_state.dispatch(key, value),
        };
    }

    pub fn ensure_state_match(&mut self) {
        assert_eq!(self.da_state.root(), self.sequencer_state.root());
    }

    pub fn revert_blocks(
        &mut self,
        state_type: StateType,
        number_of_blocks_to_revert: u64,
    ) -> Result<()> {
        let current_block_number = self.current_block_number(state_type)?;
        let target_block = current_block_number - number_of_blocks_to_revert;

        // Restore leaves to DA state
        let leaves: Leaves = self.get_leaves(state_type, target_block)?;

        match state_type {
            StateType::Sequencer => self.sequencer_state.override_state(leaves),
            StateType::DA => self.da_state.override_state(leaves),
        };

        Ok(())
    }

    pub fn trust_block(&mut self, block_number: u64) -> Result<()> {
        let Some(block_state_root) = self.sequencer_state.root() else {
            bail!("Could not compute sequencer state root");
        };

        // Set trusted Sequencer block
        self.node_state.trusted = block_state_root;

        self.update_block_storage(StateType::Sequencer, block_number)?;

        Ok(())
    }

    pub fn publish_block(&mut self, block_number: u64) -> Result<()> {
        self.node_state.on_da = self.node_state.trusted;
        self.update_block_storage(StateType::DA, block_number)?;
        Ok(())
    }

    pub fn finalize_block(&mut self) -> Result<()> {
        let Some(block_state_root) = self.da_state.root() else {
            bail!("Could not compute DA state root");
        };
        self.node_state.on_da_finalized = block_state_root;
        Ok(())
    }

    fn current_block_number(&self, state_type: StateType) -> Result<u64> {
        let key = match state_type {
            StateType::DA => "da-current-block",
            StateType::Sequencer => "seq-current-block",
        };
        let block_number_str: String =
            std::str::from_utf8(&self.storage.get(key)?.expect("Should be set"))?.into();

        Ok(block_number_str.parse::<u64>()?)
    }

    fn get_leaves(&self, state_type: StateType, target_block: u64) -> Result<Leaves> {
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

    fn update_block_storage(&mut self, state_type: StateType, block_number: u64) -> Result<()> {
        let (key, leaves) = match state_type {
            StateType::DA => ("da", &self.da_state.leaves),
            StateType::Sequencer => ("seq", &self.sequencer_state.leaves),
        };

        let leaves = serde_json::to_string(leaves)?;
        self.storage
            .insert(format!("{}-block-{}", key, block_number), leaves.as_str())?;
        self.storage.insert(
            format!("{}-current-block", key),
            block_number.to_string().as_str(),
        )?;

        Ok(())
    }
}
