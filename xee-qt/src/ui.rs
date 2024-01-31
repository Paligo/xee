use crossterm::{
    execute,
    style::{self, Stylize},
};
use std::io::{stdout, Stdout};
use std::path::Path;

use crate::outcome::{CatalogOutcomes, TestOutcome, TestSetOutcomes, UnexpectedError};
use crate::qt;
use crate::run::RunContext;
use crate::{
    error::{Error, Result},
    filter::TestFilter,
};

pub(crate) fn run(
    run_context: RunContext,
    test_filter: &impl TestFilter,
) -> Result<CatalogOutcomes> {
    let mut stdout = stdout();

    let mut catalog_outcomes = CatalogOutcomes::new();

    // XXX annoying clone
    for file_path in &run_context.catalog.file_paths.clone() {
        let test_set_outcomes = run_path_helper(run_context, test_filter, file_path, &mut stdout)?;
        catalog_outcomes.add_outcomes(test_set_outcomes);
    }
    Ok(catalog_outcomes)
}

pub(crate) fn run_path(
    mut run_context: RunContext,
    test_filter: &impl TestFilter,
    path: &Path,
) -> Result<TestSetOutcomes> {
    let mut stdout = stdout();
    run_path_helper(run_context, test_filter, path, &mut stdout)
}

fn run_path_helper(
    mut run_context: RunContext,
    test_filter: &impl TestFilter,
    path: &Path,
    stdout: &mut Stdout,
) -> Result<TestSetOutcomes> {
    if !run_context.catalog.file_paths.contains(path) {
        return Err(Error::FileNotFoundInCatalog(path.to_path_buf()));
    }
    let verbose = run_context.verbose;
    let full_path = run_context.catalog.base_dir().join(path);
    let test_set = qt::TestSet::load_from_file(run_context.xot, &full_path)?;
    if verbose {
        run_test_set(
            run_context,
            test_filter,
            &test_set,
            stdout,
            VerboseRenderer::new(),
        )
    } else {
        run_test_set(
            run_context,
            test_filter,
            &test_set,
            stdout,
            CharacterRenderer::new(),
        )
    }
}

trait Renderer {
    fn render_test_set(
        &self,
        stdout: &mut Stdout,
        test_set: &qt::TestSet,
        catalog: &qt::Catalog,
    ) -> crossterm::Result<()>;
    fn render_test_case(
        &self,
        stdout: &mut Stdout,
        test_case: &qt::TestCase,
    ) -> crossterm::Result<()>;
    fn render_test_outcome(
        &self,
        stdout: &mut Stdout,
        test_result: &TestOutcome,
    ) -> crossterm::Result<()>;
    fn render_test_set_summary(
        &self,
        stdout: &mut Stdout,
        test_set: &qt::TestSet,
    ) -> crossterm::Result<()>;
}

fn run_test_set<R: Renderer>(
    run_context: RunContext,
    test_filter: &impl TestFilter,
    test_set: &qt::TestSet,
    stdout: &mut Stdout,
    renderer: R,
) -> Result<TestSetOutcomes> {
    renderer.render_test_set(stdout, test_set, &run_context.catalog)?;

    let mut test_set_outcomes = TestSetOutcomes::new(&test_set.name);
    for test_case in &test_set.test_cases {
        if !test_filter.is_included(test_set, test_case) {
            test_set_outcomes.add_filtered();
            continue;
        }
        // skip any test case we don't support, either on test set or
        // test case level
        if !test_set
            .dependencies
            .is_supported(&run_context.known_dependencies)
            || !test_case
                .dependencies
                .is_supported(&run_context.known_dependencies)
        {
            test_set_outcomes.add_unsupported();
            continue;
        }
        renderer.render_test_case(stdout, test_case)?;
        let outcome = test_case.run(run_context, test_set);
        renderer.render_test_outcome(stdout, &outcome)?;
        test_set_outcomes.add_outcome(&test_case.name, outcome);
    }
    renderer.render_test_set_summary(stdout, test_set)?;
    Ok(test_set_outcomes)
}

struct CharacterRenderer {}

impl CharacterRenderer {
    fn new() -> Self {
        Self {}
    }
}

impl Renderer for CharacterRenderer {
    fn render_test_set(
        &self,
        _stdout: &mut Stdout,
        test_set: &qt::TestSet,
        catalog: &qt::Catalog,
    ) -> crossterm::Result<()> {
        print!("{} ", test_set.file_path(catalog).display());
        Ok(())
    }

    fn render_test_case(
        &self,
        _stdout: &mut Stdout,
        _test_case: &qt::TestCase,
    ) -> crossterm::Result<()> {
        Ok(())
    }

    fn render_test_outcome(
        &self,
        stdout: &mut Stdout,
        outcome: &TestOutcome,
    ) -> crossterm::Result<()> {
        match outcome {
            TestOutcome::Passed => {
                execute!(stdout, style::PrintStyledContent(".".green()))?;
            }
            TestOutcome::PassedWithUnexpectedError(_) => {
                execute!(stdout, style::PrintStyledContent("U".yellow()))?;
            }
            TestOutcome::Failed(_) => {
                execute!(stdout, style::PrintStyledContent("F".red()))?;
            }
            TestOutcome::RuntimeError(_) => {
                execute!(stdout, style::PrintStyledContent("E".red()))?;
            }
            TestOutcome::CompilationError(_) => {
                execute!(stdout, style::PrintStyledContent("E".red()))?;
            }
            TestOutcome::UnsupportedExpression(_) => {
                execute!(stdout, style::PrintStyledContent("E".red()))?;
            }
            TestOutcome::Unsupported => {
                execute!(stdout, style::PrintStyledContent("E".red()))?;
            }
            TestOutcome::EnvironmentError(_) => {
                execute!(stdout, style::PrintStyledContent("E".red()))?;
            }
        }
        Ok(())
    }

    fn render_test_set_summary(
        &self,
        _stdout: &mut Stdout,
        _test_set: &qt::TestSet,
    ) -> crossterm::Result<()> {
        println!();
        Ok(())
    }
}

struct VerboseRenderer {}

impl VerboseRenderer {
    fn new() -> Self {
        Self {}
    }
}

impl Renderer for VerboseRenderer {
    fn render_test_set(
        &self,
        _stdout: &mut Stdout,
        test_set: &qt::TestSet,
        catalog: &qt::Catalog,
    ) -> crossterm::Result<()> {
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
        test_case: &qt::TestCase,
    ) -> crossterm::Result<()> {
        print!("{} ... ", test_case.name);
        Ok(())
    }

    fn render_test_outcome(
        &self,
        _stdout: &mut Stdout,
        test_result: &TestOutcome,
    ) -> crossterm::Result<()> {
        println!("{}", test_result);
        Ok(())
    }

    fn render_test_set_summary(
        &self,
        _stdout: &mut Stdout,
        _test_set: &qt::TestSet,
    ) -> crossterm::Result<()> {
        println!();
        Ok(())
    }
}

impl std::fmt::Display for TestSetOutcomes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for test_outcome in self.outcomes.iter() {
            if test_outcome.outcome.is_exactly_passed() {
                continue;
            }
            writeln!(
                f,
                "{} ... {}",
                test_outcome.test_case_name, test_outcome.outcome
            )?;
        }
        Ok(())
    }
}

impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestOutcome::Passed => write!(f, "{}", "PASS".green()),
            TestOutcome::PassedWithUnexpectedError(error) => match error {
                UnexpectedError::Code(s) => write!(f, "{} code: {}", "WRONG ERROR".yellow(), s),
                UnexpectedError::Error(e) => {
                    write!(f, "{} error: {}", "WRONG ERROR".yellow(), e)
                }
            },
            TestOutcome::Failed(failure) => {
                write!(f, "{} {}", "FAIL".red(), failure)
            }
            TestOutcome::RuntimeError(error) => {
                write!(f, "{} {} {:?}", "RUNTIME ERROR".red(), error, error)
            }
            TestOutcome::CompilationError(error) => {
                write!(f, "{} {} {:?}", "COMPILATION ERROR".red(), error, error)
            }
            TestOutcome::UnsupportedExpression(error) => {
                write!(f, "{} {}", "UNSUPPORTED EXPRESSION ERROR".red(), error)
            }
            TestOutcome::Unsupported => {
                write!(f, "{}", "UNSUPPORTED".red())
            }
            TestOutcome::EnvironmentError(error) => {
                write!(f, "{} {}", "CONTEXT ITEM ERROR".red(), error)
            }
        }
    }
}
