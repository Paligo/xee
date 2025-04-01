use anyhow::Result;
use std::io::Stdout;
use std::path::{Path, PathBuf};

use xee_xpath::{context, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable, ContextWithPath, PathLoadable};

use crate::environment::SharedEnvironments;
use crate::filter::TestFilter;
use crate::hashmap::FxIndexSet;
use crate::language::Language;
use crate::ns::XPATH_TEST_NS;
use crate::outcomes::CatalogOutcomes;
use crate::renderer::Renderer;
use crate::runcontext::RunContext;
use crate::testset::TestSet;

#[derive(Debug)]
pub(crate) struct TestSetRef {
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) file: PathBuf,
}

#[derive(Debug)]
pub(crate) struct Catalog<L: Language> {
    pub(crate) shared_environments: SharedEnvironments<L::Environment>,
    pub(crate) full_path: PathBuf,
    #[allow(dead_code)]
    pub(crate) test_suite: String,
    #[allow(dead_code)]
    pub(crate) version: String,
    // pub(crate) test_sets: Vec<TestSetRef>,
    pub(crate) file_paths: FxIndexSet<PathBuf>,
}

impl<L: Language> Catalog<L> {
    pub(crate) fn base_dir(&self) -> &Path {
        self.full_path.parent().unwrap()
    }

    pub(crate) fn run(
        &self,
        run_context: &mut RunContext,
        test_filter: &impl TestFilter<L>,
        out: &mut Stdout,
        renderer: &dyn Renderer<L>,
    ) -> Result<CatalogOutcomes> {
        let mut catalog_outcomes = CatalogOutcomes::new();
        for file_path in &self.file_paths {
            let context = LoadContext {
                path: self.base_dir().join(file_path),
            };
            let test_set = TestSet::load_from_file(&context)?;
            let test_set_outcomes = test_set.run(run_context, self, test_filter, out, renderer)?;
            catalog_outcomes.add_outcomes(test_set_outcomes);
        }
        Ok(catalog_outcomes)
    }
}

pub(crate) struct LoadContext {
    pub(crate) path: PathBuf,
}

impl ContextWithPath for LoadContext {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl<L: Language> ContextLoadable<LoadContext> for Catalog<L> {
    fn static_context_builder<'n>(context: &LoadContext) -> context::StaticContextBuilder<'n> {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(XPATH_TEST_NS);
        builder
    }

    fn load_with_context(
        queries: &Queries,
        context: &LoadContext,
    ) -> Result<impl Query<Catalog<L>>> {
        let test_suite_query = queries.one("@test-suite/string()", convert_string)?;
        let version_query = queries.one("@version/string()", convert_string)?;

        let shared_environments_query = SharedEnvironments::load_with_context(queries, context)?;

        let test_set_name_query = queries.one("@name/string()", convert_string)?;
        let test_set_file_query = queries.one("@file/string()", convert_string)?;
        let test_set_query = queries.many("test-set", move |session, item| {
            let name = test_set_name_query.execute(session, item)?;
            let file = PathBuf::from(test_set_file_query.execute(session, item)?);
            Ok(TestSetRef { name, file })
        })?;
        let catalog_query = queries.one("/catalog", move |session, item| {
            let test_suite = test_suite_query.execute(session, item)?;
            let version = version_query.execute(session, item)?;
            let shared_environments = shared_environments_query.execute(session, item)?;
            let test_sets = test_set_query.execute(session, item)?;
            let file_paths = test_sets.iter().map(|t| t.file.clone()).collect();
            Ok(Catalog {
                full_path: context.path.to_path_buf(),
                test_suite,
                version,
                shared_environments,
                // test_sets,
                file_paths,
            })
        })?;
        Ok(catalog_query)
    }
}
