use std::io;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;

use crate::Cli;

/// Generate shell completion scripts.
///
/// The completion script is written to stdout.
#[derive(Debug, Parser)]
pub(crate) struct Completion {
    /// The shell to generate completions for.
    pub(crate) shell: Shell,
}

impl Completion {
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        let mut cmd = Cli::command();
        clap_complete::generate(self.shell, &mut cmd, "xee", &mut io::stdout());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::ValueEnum;

    use super::*;

    #[test]
    fn verify_cli_for_completion() {
        Cli::command().debug_assert();
    }

    #[test]
    fn generate_completions_for_all_shells() {
        for shell in Shell::value_variants() {
            let mut cmd = Cli::command();
            let mut buf = Vec::new();
            clap_complete::generate(*shell, &mut cmd, "xee", &mut buf);
            assert!(
                !buf.is_empty(),
                "completion output for {shell} should not be empty"
            );
        }
    }
}
