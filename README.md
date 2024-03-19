# Rollup Sequencer Simulator

This application simulates state sync between the sequencer and the DA (Data availability) layer. It makes sure that the state roots match between each 5 Sequencer blocks and 1 DA block.

## Key objectives

1. Dispatch sequencer state changes
2. Match state roots for every 5 sequencer blocks against 1 DA block.
3. Handle DA re-orgs in the sequencer according to the last known DA state.
4. Handle malicious sequencer updates by reverting the last known DA state.
5. Implement `StateProvider` trait to be able to provide leaf value & finalization status either from the latest block or a specific historical block.

## Building & Running

``` sh
cargo build --release
./target/release/chainway
```

## Further optimizations

1. Prevent reconstruction of merkle tree (Library used prevents in-place update of leaf nodes). This could be done by writing a custom merkle tree implementation which enables in-place modification and root recalculation based on new values. Updating the merkle tree provided by the rs-merkle library can only be done append-only which breaks the requirement of 256 leaf nodes requirement. Therefore, the current workaround is to use a BTreeMap to efficiently read/write leaf data and then calculate the state root by reconstructing the tree every time which is inefficient.
2. Only maintain block storage up until the last finalized DA block so that further sequencer updates are rolled-back (in case needed) to the most recent finalized block.
