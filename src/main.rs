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
        dispatch_state_changes(&mut node, StateType::Sequencer, state_changes);
        node.trust_block(seq_block_reader.block_number)?;

        // After each 5 blocks, publish latest state to DA layer.
        // In this task, we're not actually publishing but more like
        // verifying that the latest state in the DA block matches the latest
        // in the sequencer state.
        if seq_block_reader.block_number % 5 == 0 {
            apply_batch_to_data_availability(
                &mut node,
                &mut da_block_reader,
                &mut seq_block_reader,
            )?;
        }

        if da_block_reader.block_number > 5 && da_block_reader.block_number % 4 == 0 {
            node.finalize_block()?;
        }
    }

    println!("Finished execution");
    println!(
        "Sequencer root: 0x{}",
        hex::encode(node.sequencer_state.root().unwrap())
    );
    println!("DA root: 0x{}", hex::encode(node.da_state.root().unwrap()));
    println!(
        "Last sequencer trusted block: {}",
        hex::encode(node.node_state.trusted)
    );
    println!(
        "DA non-finalized block: {}",
        hex::encode(node.node_state.on_da)
    );
    println!(
        "DA finalized block: {}",
        hex::encode(node.node_state.on_da_finalized)
    );

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

fn apply_batch_to_data_availability(
    node: &mut Node,
    da_block_reader: &mut BlockReader,
    seq_block_reader: &mut BlockReader,
) -> Result<()> {
    // Read DA block
    let line = da_block_reader.next().expect("Should have a line")?;

    // In case of a reorg, we should revert last N DA blocks
    // and last 5*N sequencer blocks.
    if line.starts_with("REORG") {
        let number_of_blocks_to_reorg = line
            .split_ascii_whitespace()
            .last()
            .expect("REORG should have a block number")
            .parse::<u64>()?;

        println!("Reorg DA by {} blocks", number_of_blocks_to_reorg);

        node.revert_blocks(StateType::DA, number_of_blocks_to_reorg)?;
        node.revert_blocks(StateType::Sequencer, (number_of_blocks_to_reorg + 1) * 5)?;

        da_block_reader.block_number -= number_of_blocks_to_reorg + 1;
        seq_block_reader.block_number -= (number_of_blocks_to_reorg + 1) * 5;

        for _ in 0..number_of_blocks_to_reorg {
            let Some(Ok(line)) = da_block_reader.next() else {
                panic!("Should have the re-org amount of lines");
            };
            let state_changes = get_state_changes(line);
            for (key, value) in state_changes {
                node.dispatch_state_change(StateType::DA, key, value);
                node.dispatch_state_change(StateType::Sequencer, key, value);
            }
            for _ in 0..5 {
                seq_block_reader.block_number += 1;
                node.trust_block(seq_block_reader.block_number)?;
            }
        }
        node.publish_block(da_block_reader.block_number)?;

        // Make sure we have reverted back to a state where the roots between
        // DA and Sequencer are still matching.
        assert!(node.is_state_match());
        return Ok(());
    }

    let state_changes = get_state_changes(line);
    dispatch_state_changes(node, StateType::DA, state_changes.clone());

    if !node.is_state_match() {
        // Sequencer lied about the state, we should revert the last 5 sequencer blocks.
        node.revert_blocks(StateType::Sequencer, 5)?;
        seq_block_reader.block_number -= 5;
        node.trust_block(seq_block_reader.block_number)?;

        // println!("{:?}", node.sequencer_state.leaves);
        dispatch_state_changes(node, StateType::Sequencer, state_changes);
        seq_block_reader.block_number += 5;
        node.trust_block(seq_block_reader.block_number)?;
        assert!(node.is_state_match());
    }
    // The non-finalized DA block is updated.
    node.publish_block(da_block_reader.block_number)?;

    Ok(())
}

pub fn dispatch_state_changes(
    node: &mut Node,
    state_type: StateType,
    state_changes: Vec<(u8, u64)>,
) {
    for (key, state_change) in state_changes {
        node.dispatch_state_change(state_type, key, state_change);
    }
}
