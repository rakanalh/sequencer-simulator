use anyhow::Result;
mod node;
mod readers;
mod state_machine;
use itertools::Itertools;
use node::Node;
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
            node.dispatch_sequencer_state_change(key, state_change);
        }
        node.publish_sequencer_block(seq_block_reader.block_number)?;

        // After each 5 blocks, publish latest state to DA layer.
        // In this task, we're not actually publishing but more like
        // verifying that the latest state in the DA block matches the latest
        // in the sequencer state.
        if seq_block_reader.block_number % 5 == 0 {
            apply_da_state_change(&mut node, &mut da_block_reader, true)?;
        }

        if da_block_reader.block_number % 5 == 4 {
            // Finalize block_number - 4 block.
            node.finalize_sequencer_block();
        }
    }

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
    match_state: bool,
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

        node.revert_da(number_of_blocks_to_revert)?;
        // Immediately apply the next 3 state changes
        for i in 0..number_of_blocks_to_revert {
            apply_da_state_change(node, da_block_reader, false)?;
        }
        return Ok(());
    }

    let state_changes = get_state_changes(line);
    for (key, state_change) in state_changes {
        node.dispatch_da_state_change(key, state_change);
    }
    // println!("SEQ BN {}", seq_block_reader.block_number);
    // println!("DA BN {}", da_block_reader.block_number);
    if match_state {
        node.ensure_state_match();
    }
    // The non-finalized DA block is updated.
    node.update_da_block(da_block_reader.block_number)?;

    Ok(())
}