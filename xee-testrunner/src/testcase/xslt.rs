use std::path::PathBuf;

use anyhow::Result;
use iri_string::types::IriAbsoluteString;
use xot::xmlname::OwnedName as Name;

use xee_xpath::{
    context::{self, StaticContextBuilder},
    Documents, Queries, Query,
};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::{
    catalog::{Catalog, LoadContext},
    language::XsltLanguage,
    runcontext::RunContext,
    testset::TestSet,
};

use super::{
    assert::TestCaseResult,
    core::{Runnable, TestCase},
    outcome::TestOutcome,
};

#[derive(Debug)]
pub(crate) struct XsltTestCase {
    pub(crate) test_case: TestCase<XsltLanguage>,
    pub(crate) test: XsltTest,
}

impl XsltTestCase {}

#[derive(Debug)]
pub(crate) struct XsltTest {
    pub(crate) base_dir: PathBuf,
    pub(crate) stylesheets: Vec<Stylesheet>,
    pub(crate) params: Vec<TestParam>,
    pub(crate) initial_template: Option<Name>,
}

#[derive(Debug, Clone)]
pub(crate) struct Stylesheet {
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TestParam {
    pub(crate) name: Name,
    pub(crate) select: String,
}

impl Runnable<XsltLanguage> for XsltTestCase {
    fn test_case(&self) -> &TestCase<XsltLanguage> {
        &self.test_case
    }

    fn run(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<XsltLanguage>,
        test_set: &TestSet<XsltLanguage>,
    ) -> TestOutcome {
        let stylesheet = if self.test.stylesheets.is_empty() {
            let environments = match self
                .test_case
                .environments(catalog, test_set)
                .collect::<std::result::Result<Vec<_>, crate::error::Error>>()
            {
                Ok(environments) => environments,
                Err(error) => {
                    return TestOutcome::EnvironmentError(format!(
                        "Error loading environments: {}",
                        error
                    ))
                }
            };
            let environment_stylesheet = environments
                .iter()
                .find_map(|environment| environment.stylesheets.first())
                .and_then(|stylesheet| stylesheet.path.clone());
            match environment_stylesheet {
                Some(path) => Stylesheet { path: Some(path) },
                None => {
                    return TestOutcome::EnvironmentError("No stylesheet found".to_string());
                }
            }
        } else {
            self.test.stylesheets[0].clone()
        };
        // construct full path
        let path = self.test.base_dir.join(stylesheet.path.as_ref().unwrap());
        // load xml text from file
        let f = std::fs::File::open(&path).unwrap();
        let xslt = std::io::read_to_string(f);
        let xslt = match xslt {
            Ok(xslt) => xslt,
            Err(error) => {
                return TestOutcome::EnvironmentError(format!(
                    "Error reading stylesheet: {}",
                    error
                ))
            }
        };
        // get static base URI: todo refactor out into its own function
        let static_base_uri = self.test_case.static_base_uri(catalog, test_set);
        let static_base_uri = match static_base_uri {
            Ok(static_base_uri) => static_base_uri,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };

        let static_base_uri = if let Some(static_base_uri) = static_base_uri {
            if static_base_uri != "#UNDEFINED" {
                let iri: IriAbsoluteString = static_base_uri.try_into().unwrap();
                Some(iri)
            } else {
                None
            }
        } else {
            // in the absence of an explicit base URI, we use the test file's URI
            // path of thist file
            Some(test_set.file_uri())
        };

        let mut static_context_builder = StaticContextBuilder::default();
        let assertions_enabled = !self
            .test_case
            .dependencies
            .is_feature_disabled("enable_assertions")
            && !test_set
                .dependencies
                .is_feature_disabled("enable_assertions");
        static_context_builder.assertions_enabled(assertions_enabled);
        let variables =
            self.test_case
                .variables(run_context, catalog, test_set, static_base_uri.as_deref());
        let mut variables = match variables {
            Ok(variables) => variables,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };
        for param in &self.test.params {
            let queries = Queries::default();
            let query = match queries.sequence(&param.select) {
                Ok(query) => query,
                Err(error) => {
                    return TestOutcome::EnvironmentError(format!(
                        "param: select xpath parse failed: {}",
                        error
                    ))
                }
            };
            let mut documents = Documents::new();
            let dynamic_context_builder = query.dynamic_context_builder(&documents);
            let dynamic_context = dynamic_context_builder.build();
            let result = match query.execute_with_context(&mut documents, &dynamic_context) {
                Ok(result) => result,
                Err(error) => {
                    return TestOutcome::EnvironmentError(format!(
                        "param: select xpath eval failed: {}",
                        error
                    ))
                }
            };
            variables.insert(param.name.clone(), result);
        }
        let variable_names: Vec<_> = variables.keys().cloned().collect();
        static_context_builder.variable_names(variable_names);
        let static_context = static_context_builder.build();
        let program = xee_xslt_compiler::parse_with_base(static_context, &xslt, Some(&path));
        let program = match program {
            Ok(program) => program,
            Err(error) => {
                return match &self.test_case.result {
                    TestCaseResult::AssertError(assert_error) => {
                        assert_error.assert_error(&error.error)
                    }
                    TestCaseResult::AnyOf(any_of) => any_of.assert_error(&error.error),
                    _ => TestOutcome::CompilationError(error.error),
                }
            }
        };

        // let root = run_context.documents.xot().parse(xml).unwrap();

        // load all the sources
        // this makes the sources available on the appropriate URLs
        let r =
            self.test_case
                .load_sources(run_context, catalog, test_set, static_base_uri.as_deref());
        match r {
            Ok(_) => (),
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        }

        // the context item is loaded
        let context_item =
            self.test_case
                .context_item(run_context, catalog, test_set, static_base_uri.as_deref());
        let context_item = match context_item {
            Ok(context_item) => context_item,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };

        // now construct the dynamic context. We want to have one here
        // explicitly so we can use it later in the assertions
        let mut builder = program.dynamic_context_builder();
        if let Some(context_item) = context_item {
            builder.context_item(context_item);
        }
        builder.documents(run_context.documents.documents().clone());
        builder.type_table(run_context.documents.type_table().clone());
        builder.variables(variables.clone());
        builder.current_datetime(chrono::offset::Utc::now().into());
        let context = builder.build();
        let runnable = program.runnable(&context);
        let result = if let Some(initial_template) = &self.test.initial_template {
            runnable.call_named_template(
                run_context.documents.xot_mut(),
                initial_template,
                None,
            )
        } else {
            runnable.many(run_context.documents.xot_mut())
        };

        self.test_case.result.assert_result(
            &context,
            run_context.documents,
            &result.map_err(|error| error.error),
        )
    }

    fn load(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        XsltTestCase::load_with_context(queries, context)
    }
}

impl ContextLoadable<LoadContext> for XsltTestCase {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder<'_> {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        let file_query = queries.option("@file/string()", convert_string)?;
        let stylesheets_query = queries.many("stylesheet", move |documents, item| {
            let file = file_query.execute(documents, item)?;
            Ok(Stylesheet { path: file })
        })?;
        let param_name_query = queries.one("@name/string()", convert_string)?;
        let param_select_query = queries.one("@select/string()", convert_string)?;
        let params_query = queries.many("param", move |documents, item| {
            let name = param_name_query.execute(documents, item)?;
            let select = param_select_query.execute(documents, item)?;
            Ok(TestParam {
                name: Name::name(&name),
                select,
            })
        })?;

        let initial_template_query =
            queries.option("initial-template/@name/string()", convert_string)?;
        let xslt_test_query = queries.one("test", move |documents, item| {
            // the base dir is the same as the test set path, but
            // without the filename
            let base_dir = context.path.parent().unwrap();

            let stylesheets = stylesheets_query.execute(documents, item)?;
            let params = params_query.execute(documents, item)?;
            let initial_template = initial_template_query.execute(documents, item)?;
            let initial_template = initial_template.map(|name| Name::name(&name));
            Ok(XsltTest {
                stylesheets,
                params,
                base_dir: base_dir.to_path_buf(),
                initial_template,
            })
        })?;
        let test_case_query = TestCase::load_with_context(queries, context)?;
        let xslt_test_case_query = queries.one(".", move |documents, item| {
            let test_case = test_case_query.execute(documents, item)?;
            let xslt_test = xslt_test_query.execute(documents, item)?;
            Ok(XsltTestCase {
                test_case,
                test: xslt_test,
            })
        })?;

        Ok(xslt_test_case_query)
    }
}
