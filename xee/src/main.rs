mod common;
mod error;
mod format;
mod indent;
mod repl;
mod repl_cmd;
mod xpath;
mod xslt;

use clap::{Parser, Subcommand};

pub(crate) const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " ", env!("GIT_COMMIT"));

#[derive(Parser)]
#[command(author, about, version=VERSION, long_about)]
pub(crate) struct Cli {
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
