use std::io;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use anyhow::Context;

/// Reads XML input from a file or stdin.
pub(crate) fn input_xml(infile: &Option<PathBuf>) -> anyhow::Result<String> {
    if let Some(input_path) = infile {
        std::fs::read_to_string(input_path).with_context(|| {
            format!("Failed to read input XML file: {}", input_path.display())
        })
    } else {
        // Read from stdin if no input file is provided
        let mut input_reader = BufReader::new(io::stdin());
        let mut input_xml = String::new();
        input_reader
            .read_to_string(&mut input_xml)
            .context("Failed to read XML from stdin")?;
        Ok(input_xml)
    }
}