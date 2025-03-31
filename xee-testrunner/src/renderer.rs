use std::io::{IsTerminal, Stdout, Write};

use crossterm::{
    execute,
    style::{self, Stylize},
};

use crate::{
    catalog::Catalog,
    language::Language,
    testcase::{TestCase, TestOutcome, UnexpectedError},
    testset::TestSet,
};

pub(crate) trait Renderer<L: Language> {
    fn render_test_set(
        &self,
        stdout: &mut Stdout,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
    ) -> std::io::Result<()>;
    fn render_test_case(&self, stdout: &mut Stdout, test_case: &TestCase<L>)
        -> std::io::Result<()>;
    fn render_test_outcome(
        &self,
        stdout: &mut Stdout,
        test_result: &TestOutcome,
    ) -> std::io::Result<()>;
    fn render_test_set_summary(
        &self,
        stdout: &mut Stdout,
        test_set: &TestSet<L>,
    ) -> std::io::Result<()>;
}

pub(crate) struct VerboseRenderer {}

impl VerboseRenderer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl<L: Language> Renderer<L> for VerboseRenderer {
    fn render_test_set(
        &self,
        _stdout: &mut Stdout,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
    ) -> std::io::Result<()> {
        println!("{}", test_set.file_path(catalog).display());
        println!("{}", test_set.name);
        for description in &test_set.descriptions {
            println!("{} ", description);
        }
        Ok(())
    }

    fn render_test_case(
        &self,
        _stdout: &mut Stdout,
        test_case: &TestCase<L>,
    ) -> std::io::Result<()> {
        print!("{} ... ", test_case.name);
        Ok(())
    }

    fn render_test_outcome(
        &self,
        stdout: &mut Stdout,
        test_result: &TestOutcome,
    ) -> std::io::Result<()> {
        match test_result {
            TestOutcome::Passed => {
                writeln!(
                    stdout,
                    "{}",
                    with_color(stdout, "PASS", crossterm::style::Color::Green)
                )
            }
            TestOutcome::UnexpectedError(error) => match error {
                UnexpectedError(s) => writeln!(
                    stdout,
                    "{} code: {}",
                    with_color(stdout, "WRONG ERROR", crossterm::style::Color::Yellow),
                    s
                ),
            },
            TestOutcome::Failed(failure) => {
                writeln!(
                    stdout,
                    "{} {}",
                    with_color(stdout, "FAIL", crossterm::style::Color::Red),
                    failure
                )
            }
            TestOutcome::RuntimeError(error) => {
                writeln!(
                    stdout,
                    "{} {} {:?}",
                    with_color(stdout, "RUNTIME ERROR", crossterm::style::Color::Red),
                    error,
                    error
                )
            }
            TestOutcome::CompilationError(error) => {
                writeln!(
                    stdout,
                    "{} {} {:?}",
                    with_color(stdout, "COMPILATION ERROR", crossterm::style::Color::Red),
                    error,
                    error
                )
            }
            TestOutcome::UnsupportedExpression(error) => {
                writeln!(
                    stdout,
                    "{} {}",
                    with_color(
                        stdout,
                        "UNSUPPORTED EXPRESSION ERROR",
                        crossterm::style::Color::Red
                    ),
                    error
                )
            }
            TestOutcome::Unsupported => {
                writeln!(
                    stdout,
                    "{}",
                    with_color(stdout, "UNSUPPORTED", crossterm::style::Color::Red)
                )
            }
            TestOutcome::EnvironmentError(error) => {
                writeln!(
                    stdout,
                    "{} {}",
                    with_color(stdout, "CONTEXT ITEM ERROR", crossterm::style::Color::Red),
                    error
                )
            }
        }
    }

    fn render_test_set_summary(
        &self,
        _stdout: &mut Stdout,
        _test_set: &TestSet<L>,
    ) -> std::io::Result<()> {
        println!();
        Ok(())
    }
}

pub(crate) struct CharacterRenderer {}

impl CharacterRenderer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl<L: Language> Renderer<L> for CharacterRenderer {
    fn render_test_set(
        &self,
        _stdout: &mut Stdout,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
    ) -> std::io::Result<()> {
        print!("{} ", test_set.file_path(catalog).display());
        Ok(())
    }

    fn render_test_case(
        &self,
        _stdout: &mut Stdout,
        _test_case: &TestCase<L>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    fn render_test_outcome(
        &self,
        stdout: &mut Stdout,
        outcome: &TestOutcome,
    ) -> std::io::Result<()> {
        match outcome {
            TestOutcome::Passed => render_error_code(stdout, ".", crossterm::style::Color::Green),
            TestOutcome::UnexpectedError(_) => {
                render_error_code(stdout, "F", crossterm::style::Color::Red)
            }
            TestOutcome::Failed(_) => render_error_code(stdout, "F", crossterm::style::Color::Red),
            TestOutcome::RuntimeError(_) => {
                render_error_code(stdout, "E", crossterm::style::Color::Red)
            }
            TestOutcome::CompilationError(_) => {
                render_error_code(stdout, "E", crossterm::style::Color::Red)
            }
            TestOutcome::UnsupportedExpression(_) => {
                render_error_code(stdout, "E", crossterm::style::Color::Red)
            }
            TestOutcome::Unsupported => {
                render_error_code(stdout, "E", crossterm::style::Color::Red)
            }
            TestOutcome::EnvironmentError(_) => {
                render_error_code(stdout, "E", crossterm::style::Color::Red)
            }
        }
    }

    fn render_test_set_summary(
        &self,
        _stdout: &mut Stdout,
        _test_set: &TestSet<L>,
    ) -> std::io::Result<()> {
        println!();
        Ok(())
    }
}

fn render_error_code(
    stdout: &mut Stdout,
    error_code: &str,
    color: crossterm::style::Color,
) -> std::io::Result<()> {
    if stdout.is_terminal() {
        execute!(stdout, style::PrintStyledContent(error_code.with(color)))?;
    } else {
        stdout.write_all(error_code.as_bytes())?;
    }
    Ok(())
}

fn with_color<'a>(
    stdout: &Stdout,
    text: &'a str,
    color: crossterm::style::Color,
) -> crossterm::style::StyledContent<&'a str> {
    if stdout.is_terminal() {
        text.with(color)
    } else {
        text.stylize()
    }
}
