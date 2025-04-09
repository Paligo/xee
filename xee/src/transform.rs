use std::io::{self, BufReader, Read};
use std::path::PathBuf;

use crate::error::render_error;
use anyhow::Context;
use clap::Parser;
use xee_interpreter::sequence::Sequence;
use xee_xslt_compiler;
use xot::Xot;

#[derive(Debug, Parser)]
pub(crate) struct Transform {
    /// XSLT stylesheet file
    pub(crate) stylesheet: PathBuf,

    /// Input XML file (or use stdin if not provided)
    pub(crate) input: Option<PathBuf>,

    /// Output file (default stdout)
    #[arg(long, short)]
    pub(crate) output: Option<PathBuf>,
}

impl Transform {
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        // Read the XSLT stylesheet
        let stylesheet = std::fs::read_to_string(&self.stylesheet).with_context(|| {
            format!(
                "Failed to read stylesheet file: {}",
                self.stylesheet.display()
            )
        })?;

        // Read the input XML
        let xml = if let Some(input_path) = &self.input {
            std::fs::read_to_string(input_path).with_context(|| {
                format!("Failed to read input XML file: {}", input_path.display())
            })?
        } else {
            // Read from stdin if no input file is provided
            let mut input_reader = BufReader::new(io::stdin());
            let mut input_xml = String::new();
            input_reader
                .read_to_string(&mut input_xml)
                .context("Failed to read XML from stdin")?;
            input_xml
        };

        // Perform the XSLT transformation
        let mut xot = Xot::new();
        let result = match xee_xslt_compiler::evaluate(&mut xot, &xml, &stylesheet) {
            Ok(result) => result,
            Err(e) => {
                render_error(&stylesheet, e);
                return Ok(());
            }
        };

        // Convert result to string
        let output_str = serialize_result(&mut xot, result)?;

        // Output the result
        if let Some(output_path) = &self.output {
            std::fs::write(output_path, output_str).with_context(|| {
                format!("Failed to write output to file: {}", output_path.display())
            })?;
        } else {
            println!("{}", output_str);
        }

        Ok(())
    }
}

fn serialize_result(xot: &mut Xot, result: Sequence) -> anyhow::Result<String> {
    let mut output = String::new();

    for item in result.iter() {
        match item.to_node() {
            Ok(node) => {
                let node_str = xot
                    .to_string(node)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize node: {}", e))?;
                output.push_str(&node_str);
            }
            Err(_) => {
                // Handle non-node items (atomics, etc.)
                match item.to_atomic() {
                    Ok(atomic) => {
                        // Try to convert atomic to string
                        match atomic.to_string() {
                            Ok(s) => output.push_str(&s),
                            Err(_) => output.push_str(&format!("{:?}", atomic)),
                        }
                    }
                    Err(_) => {
                        // Just use debug formatting for other items
                        output.push_str(&format!("{:?}", item));
                    }
                }
            }
        }
    }

    Ok(output)
}
