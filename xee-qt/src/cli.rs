use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use xot::Xot;

use crate::error::Result;
use crate::filter::{ExcludedNamesFilter, IncludeAllFilter, NameFilter};
use crate::outcome::Outcomes;
use crate::path::paths;
use crate::qt;
use crate::run::RunContextBuilder;
use crate::ui::{run, run_path};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the filter.
    ///
    /// Runs all tests, and saves the test outcomes to a `filters` file in the
    /// directory of the `catalog.xml`.
    Initialize {
        /// A path to a qttests directory or individual test file
        path: PathBuf,
        /// Verbose mode
        #[clap(short, long)]
        verbose: bool,
    },
    /// Check with filters engaged.
    ///
    /// Runs tests that are not excluded by the `filters` file. This can be
    /// used to check for regressions after making changes.
    Check {
        /// A path to a qttests directory or individual test file.
        ///
        /// If individual test file, it runs only the tests in that file,
        /// otherwise it runs the tests in the `catalog.xml`.
        path: PathBuf,
        /// Verbose mode
        #[clap(short, long)]
        verbose: bool,
    },
    /// Update the filters.
    ///
    /// This runs all the tests on the path, and then updates the `filters`
    /// file if more tests pass in this run. If not, the filters file is not
    /// updated, so that you can't accidentally introduce a regression.
    Update {
        /// A path to a qttests directory or individual test file.
        ///
        /// If individual test file, it updates only the tests in that file,
        /// otherwise it updates tests in the `catalog.xml`.
        path: PathBuf,
        /// Verbose mode
        #[clap(short, long)]
        verbose: bool,
    },
    /// Run all tests.
    ///
    /// Do not use filters, simply run all the tests indicated by path.
    All {
        /// A path to a qttests directory or individual test file.
        path: PathBuf,
        /// Name filter, only test-cases that contain this name are found.
        name_filter: Option<String>,
        /// Verbose mode
        #[clap(short, long)]
        verbose: bool,
    },
}

pub fn cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Initialize { path, verbose } => initialize(&path, verbose),
        Commands::Check { path, verbose } => check(&path, verbose),
        Commands::Update { path, verbose } => update(&path, verbose),
        Commands::All {
            path,
            verbose,
            name_filter,
        } => all(&path, verbose, name_filter),
    }
}

fn check(path: &Path, verbose: bool) -> Result<()> {
    let path_info = paths(path)?;
    let mut xot = Xot::new();
    let catalog = qt::Catalog::load_from_file(&mut xot, &path_info.catalog_path)?;

    let mut run_context = RunContextBuilder::default()
        .xot(xot)
        .catalog(catalog)
        .parsers(Rc::new(xee_xpath::Parsers::new()))
        .verbose(verbose)
        .build()
        .unwrap();

    if !path_info.filter_path.exists() {
        // we cannot check if we don't have a filter file yet
        println!("Cannot check without filter file");
        return Ok(());
    }

    let test_filter = ExcludedNamesFilter::load_from_file(&path_info.filter_path)?;
    if path_info.whole_catalog() {
        let outcomes = run(&mut run_context, &test_filter)?;
        println!("{}", outcomes.display());
    } else {
        let outcomes = run_path(run_context, &test_filter, &path_info.relative_path)?;
        println!("{}", outcomes.display());
    }
    Ok(())
}

fn all(path: &Path, verbose: bool, name_filter: Option<String>) -> Result<()> {
    let path_info = paths(path)?;
    let mut xot = Xot::new();
    let catalog = qt::Catalog::load_from_file(&mut xot, &path_info.catalog_path)?;

    let mut run_context = RunContextBuilder::default()
        .xot(xot)
        .catalog(catalog)
        .verbose(verbose)
        .build()
        .unwrap();

    let test_filter = NameFilter::new(name_filter);

    if path_info.whole_catalog() {
        let outcomes = run(&mut run_context, &test_filter)?;
        println!("{}", outcomes.display());
    } else {
        let outcomes = run_path(run_context, &test_filter, &path_info.relative_path)?;
        println!("{}", outcomes.display());
    }
    Ok(())
}

fn update(path: &Path, verbose: bool) -> Result<()> {
    let path_info = paths(path)?;
    let mut xot = Xot::new();
    let catalog = qt::Catalog::load_from_file(&mut xot, &path_info.catalog_path)?;

    let mut run_context = RunContextBuilder::default()
        .xot(xot)
        .catalog(catalog)
        .verbose(verbose)
        .build()
        .unwrap();

    if !path_info.filter_path.exists() {
        // we cannot update if we don't have a filter file yet
        println!("Cannot update without filter file");
        return Ok(());
    }
    let test_filter = IncludeAllFilter::new();
    let mut update_filter = ExcludedNamesFilter::load_from_file(&path_info.filter_path)?;
    if path_info.whole_catalog() {
        let catalog_outcomes = run(&mut run_context, &test_filter)?;

        update_filter.update_with_catalog_outcomes(&catalog_outcomes);
        println!("{}", catalog_outcomes.display());
    } else {
        let test_set_outcomes = run_path(run_context, &test_filter, &path_info.relative_path)?;
        update_filter.update_with_test_set_outcomes(&test_set_outcomes);
        println!("{}", test_set_outcomes.display());
    }

    let filter_data = update_filter.to_string();
    fs::write(&path_info.filter_path, filter_data)?;
    Ok(())
}

fn initialize(path: &Path, verbose: bool) -> Result<()> {
    let path_info = paths(path)?;
    if path_info.filter_path.exists() {
        println!("Cannot reinitialize filters. Use update or delete filters file first");
        return Ok(());
    }

    let mut xot = Xot::new();
    let catalog = qt::Catalog::load_from_file(&mut xot, &path_info.catalog_path)?;

    let mut run_context = RunContextBuilder::default()
        .xot(xot)
        .catalog(catalog)
        .verbose(verbose)
        .build()
        .unwrap();

    let test_filter = IncludeAllFilter::new();

    let catalog_outcomes = run(&mut run_context, &test_filter)?;

    let test_filter = ExcludedNamesFilter::from_outcomes(&catalog_outcomes);
    let filter_data = test_filter.to_string();
    fs::write(&path_info.filter_path, filter_data)?;
    Ok(())
}
