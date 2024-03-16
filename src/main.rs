use anyhow::Result;
mod node;
mod readers;
mod state_machine;
use itertools::Itertools;
use node::{Node, StateType};
use readers::BlockReader;

fn main() -> Result<()> {
    let storage = sled::open("chainway.db")?;

    let mut seq_block_reader = BlockReader::new("from_sequencer.txt")?;
    let mut da_block_reader = BlockReader::new("from_da.txt")?;

    let mut node = Node::new(storage);

    while let Some(line) = seq_block_reader.next() {
        // Dispatch sequencer state changes
        let state_changes = get_state_changes(line?);
        for (key, state_change) in state_changes {
            node.dispatch_state_change(node::StateType::Sequencer, key, state_change);
        }
        node.trust_block(seq_block_reader.block_number)?;

        // After each 5 blocks, publish latest state to DA layer.
        // In this task, we're not actually publishing but more like
        // verifying that the latest state in the DA block matches the latest
        // in the sequencer state.
        if seq_block_reader.block_number % 5 == 0 {
            apply_da_state_change(&mut node, &mut da_block_reader, &mut seq_block_reader)?;
        }

        if da_block_reader.block_number > 5 && da_block_reader.block_number % 4 == 0 {
            node.finalize_block()?;
        }
    }

    // TODO: Print out the final roots in node state

    Ok(())
}

fn get_state_changes(line: String) -> Vec<(u8, u64)> {
    let mut state_changes = vec![];
    for state_change in line.split(", ") {
        let Some((key, state_change)) = state_change.split_ascii_whitespace().collect_tuple()
        else {
            panic!("Invalid state change");
        };

        state_changes.push((
            key.parse::<u8>().unwrap(),
            state_change.parse::<u64>().unwrap(),
        ));
    }
    state_changes
}

fn apply_da_state_change(
    node: &mut Node,
    da_block_reader: &mut BlockReader,
    seq_block_reader: &mut BlockReader,
) -> Result<()> {
    // Read DA block
    let line = da_block_reader.next().expect("Should have a line")?;

    // In case of a reorg, we should revert last N DA blocks
    // and last 5*N sequencer blocks.
    if line.starts_with("REORG") {
        let number_of_blocks_to_revert = line
            .split_ascii_whitespace()
            .last()
            .expect("REORG should have a block number")
            .parse::<u64>()?;

        node.revert_blocks(StateType::DA, number_of_blocks_to_revert)?;
        node.revert_blocks(StateType::Sequencer, (number_of_blocks_to_revert + 1) * 5)?;

        da_block_reader.block_number -= number_of_blocks_to_revert + 1;
        seq_block_reader.block_number -= (number_of_blocks_to_revert + 1) * 5;

        // Make sure we have reverted back to a state where the roots between
        // DA and Sequencer are still matching.
        node.ensure_state_match();
        println!("FINISHED REORG");
        return Ok(());
    }

    let state_changes = get_state_changes(line);
    for (key, state_change) in state_changes {
        node.dispatch_state_change(node::StateType::DA, key, state_change);
    }
    node.ensure_state_match();
    // The non-finalized DA block is updated.
    node.publish_block(da_block_reader.block_number)?;

    Ok(())
}
