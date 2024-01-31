use globset::{Glob, GlobSet, GlobSetBuilder};
use std::env;
use std::path::PathBuf;
use xot::Xot;

use crate::error::{Error, Result};
use crate::outcome::{TestOutcome, TestSetOutcomes};
use crate::{path::paths, qt, run::RunContext, run::RunContextBuilder};

pub struct Tests {
    path: String,
    include: Vec<String>,
    exclude: Vec<String>,
    tolerate_wrong_error: bool,
}

impl Tests {
    pub fn none(path: &str) -> Self {
        Self {
            path: path.to_string(),
            include: Vec::new(),
            exclude: Vec::new(),
            tolerate_wrong_error: false,
        }
    }

    pub fn all(path: &str) -> Self {
        Self {
            path: path.to_string(),
            include: vec!["*".to_string()],
            exclude: Vec::new(),
            tolerate_wrong_error: false,
        }
    }

    pub fn include(mut self, include: &str) -> Self {
        self.include
            .extend(include.split_whitespace().map(|s| s.to_string()));
        self
    }

    pub fn exclude(mut self, exclude: &str) -> Self {
        self.exclude
            .extend(exclude.split_whitespace().map(|s| s.to_string()));
        self
    }

    pub fn bug(self, exclude: &str) -> Self {
        self.exclude(exclude)
    }

    pub fn tolerate_wrong_error(mut self) -> Self {
        self.tolerate_wrong_error = true;
        self
    }

    fn prepare(&self) -> Result<(PathBuf, RunContext)> {
        // This environment variable only exists because of the .cargo/config.toml
        // hack described here https://github.com/rust-lang/cargo/issues/3946'
        let workspace_dir = env::var("CARGO_WORKSPACE_DIR")?;
        let workspace_path = PathBuf::from(&workspace_dir);
        let qt3tests_path = workspace_path.join("vendor/qt3tests");
        let path = qt3tests_path.join(&self.path);
        let path = path.with_extension("xml");
        let path_info = paths(&path)?;
        let mut xot = Xot::new();
        let catalog = qt::Catalog::load_from_file(xot, &path_info.catalog_path)?;
        let run_context = RunContextBuilder::default()
            .xot(xot)
            .catalog(catalog)
            .verbose(false)
            .build()
            .unwrap();
        let full_path = run_context.catalog.base_dir().join(path_info.relative_path);
        Ok((full_path, run_context))
    }

    fn include_glob_set(&self) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for include in self.include.iter() {
            builder.add(Glob::new(include)?);
        }
        Ok(builder.build()?)
    }

    fn exclude_glob_set(&self) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for exclude in self.exclude.iter() {
            builder.add(Glob::new(exclude)?);
        }
        Ok(builder.build()?)
    }

    fn is_passed(&self, outcome: &TestOutcome) -> bool {
        if self.tolerate_wrong_error {
            outcome.is_passed()
        } else {
            outcome.is_exactly_passed()
        }
    }

    fn try_run(&self) -> Result<()> {
        let include_glob_set = self.include_glob_set()?;
        let exclude_glob_set = self.exclude_glob_set()?;
        let (path, mut run_context) = self.prepare()?;
        let test_set = qt::TestSet::load_from_file(run_context.xot, &path)?;
        let mut test_set_outcomes = TestSetOutcomes::new(&test_set.name);
        for test_case in &test_set.test_cases {
            if !test_case
                .dependencies
                .is_supported(&run_context.known_dependencies)
            {
                continue;
            }
            if !include_glob_set.is_match(&test_case.name)
                || exclude_glob_set.is_match(&test_case.name)
            {
                continue;
            }
            let outcome = test_case.run(run_context, &test_set);
            test_set_outcomes.add_outcome(&test_case.name, outcome);
        }
        if test_set_outcomes.has_failures() {
            Err(Error::TestFailures(path, test_set_outcomes))
        } else {
            Ok(())
        }
    }

    pub fn run(&self) {
        match self.try_run() {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }
}

fn try_test_all(path: &str) -> Result<()> {
    // This environment variable only exists because of the .cargo/config.toml
    // hack described here https://github.com/rust-lang/cargo/issues/3946'
    let workspace_dir = env::var("CARGO_WORKSPACE_DIR")?;
    let workspace_path = PathBuf::from(&workspace_dir);
    let qt3tests_path = workspace_path.join("vendor/qt3tests");
    let path = qt3tests_path.join(path);
    let path = path.with_extension("xml");
    let path_info = paths(&path)?;
    let mut xot = Xot::new();
    let catalog = qt::Catalog::load_from_file(xot, &path_info.catalog_path)?;
    let mut run_context = RunContextBuilder::default()
        .xot(xot)
        .catalog(catalog)
        .verbose(false)
        .build()
        .unwrap();
    let full_path = run_context
        .catalog
        .base_dir()
        .join(&path_info.relative_path);
    let test_set = qt::TestSet::load_from_file(run_context.xot, &full_path)?;
    let mut test_set_outcomes = TestSetOutcomes::new(&test_set.name);

    for test_case in &test_set.test_cases {
        if !test_case
            .dependencies
            .is_supported(&run_context.known_dependencies)
        {
            continue;
        }
        let outcome = test_case.run(run_context, &test_set);
        test_set_outcomes.add_outcome(&test_case.name, outcome);
    }
    if test_set_outcomes.has_failures() {
        Err(Error::TestFailures(path, test_set_outcomes))
    } else {
        Ok(())
    }
}

pub fn test_all(path: &str) {
    match try_test_all(path) {
        Ok(_) => {}
        Err(e) => panic!("{}", e),
    }
}
