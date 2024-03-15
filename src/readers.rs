use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Result},
};

pub struct BlockReader {
    pub block_number: u64,
    lines: Lines<BufReader<File>>,
}

impl BlockReader {
    pub fn new(file_name: &str) -> Result<Self> {
        let file = File::open(file_name)?;
        let reader = BufReader::new(file);

        Ok(Self {
            block_number: 0,
            lines: reader.lines(),
        })
    }

    pub fn next(&mut self) -> Option<Result<String>> {
        self.block_number += 1;
        self.lines.next()
    }
}
