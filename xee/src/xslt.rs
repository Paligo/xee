use std::path::PathBuf;

use crate::error::render_error;
use anyhow::Context;
use clap::Parser;
use xee_xslt_compiler;
use xot::Xot;
use crate::common::input_xml;
use xee_interpreter::sequence::SerializationParameters;

#[derive(Debug, Parser)]
pub(crate) struct Xslt {
    /// XSLT stylesheet file
    pub(crate) stylesheet: PathBuf,

    /// Input XML file (or use stdin if not provided)
    pub(crate) infile: Option<PathBuf>,

    /// Output file (default stdout)
    #[arg(long, short)]
    pub(crate) output: Option<PathBuf>,
}

impl Xslt {
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        // Read the XSLT stylesheet
        let stylesheet = std::fs::read_to_string(&self.stylesheet).with_context(|| {
            format!(
                "Failed to read stylesheet file: {}",
                self.stylesheet.display()
            )
        })?;

        // Read the input XML
        let xml = input_xml(&self.infile)?;

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
        let output_str = result.serialize(SerializationParameters::new(), &mut xot)?;//serialize_result(&mut xot, result)?;

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