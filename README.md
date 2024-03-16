# Rollup Sequencer Simulator

This application simulates state sync between the sequencer and the DA (Data availability) layer. It makes sure that the state roots match between each 5 Sequencer blocks and 1 DA block.

## Building

``` sh
cargo build --release
./target/release/chainway
```

## Open questions
The `generator.py` file generates the state changes to be dispatched into the sequencer. Each 5 sequencer blocks are aggregated into 1 DA block. However, the `REORG N` says that the last N DA blocks are invalid. Which means that we have to rollback N*5 sequencer blocks. The code writtein in `generator.py` writes N lines after `REORG` in which it fills with random values. This breaks the state sync correctness between the sequencer and DA layer.

The question is, when a REORG happens, what would the next random lines mean for the sequencer state?
