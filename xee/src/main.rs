mod error;
mod format;
mod indent;
mod repl;
mod repl_cmd;
mod xslt;
mod xpath;
mod common;

use clap::{Parser, Subcommand};
use const_format::formatcp;

pub(crate) const VERSION: &str = formatcp!(
    "{} ({}, {})",
    clap::crate_version!(),
    env!("SOURCE_TIMESTAMP"),
    env!("GIT_COMMIT")
);

#[derive(Parser)]
#[command(author, about,  version = VERSION, long_about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format an XML document with various options.
    Format(format::Format),
    /// Format an XML document with indentation to make it more readable.
    ///
    /// This is a shortcut for `format --indent`.
    Indent(indent::Indent),
    /// Evaluate an xpath expression on an xml document.
    Xpath(xpath::XPath),
    /// Interactive xpath REPL (read-eval-print loop).
    Repl(repl::Repl),
    /// Transform an XML document using an XSLT stylesheet.
    Xslt(xslt::Xslt),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Indent(indent) => {
            indent.run()?;
        }
        Commands::Format(format) => {
            format.run()?;
        }
        Commands::Xpath(xpath) => {
            xpath.run()?;
        }
        Commands::Repl(repl) => {
            repl.run()?;
        }
        Commands::Xslt(xslt) => {
            xslt.run()?;
        }
    }
    Ok(())
}
