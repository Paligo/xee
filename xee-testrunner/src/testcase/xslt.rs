use std::path::PathBuf;

use anyhow::Result;
use iri_string::types::IriAbsoluteString;

use xee_name::Name;
use xee_xpath::{
    context::{self, StaticContextBuilder, Variables},
    function::{Function, Map},
    Documents, Item, Queries, Query,
};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::{
    catalog::{Catalog, LoadContext},
    language::XsltLanguage,
    runcontext::RunContext,
    testset::TestSet,
};

use super::{
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
    pub(crate) params: Vec<Param>,
}

#[derive(Debug)]
pub(crate) struct Stylesheet {
    pub(crate) path: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Param {
    pub(crate) name: Option<String>,
    pub(crate) select: Option<String>,
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
        // TODO take the first stylesheet for now
        if self.test.stylesheets.is_empty() {
            return TestOutcome::EnvironmentError("No stylesheet found".to_string());
        }
        let stylesheet = &self.test.stylesheets[0];
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
        // HACK: collect param values into a map
        let mut params = Vec::<(xee_xpath::Atomic, xee_xpath::Sequence)>::new();
        {
            let mut documents = Documents::new();
            for param in &self.test.params {
                let select = (param.select.as_ref()).expect("param: missing select not supported");
                let name = (param.name.as_ref()).expect("param: missing name not supported");
                let queries = Queries::default();

                let query = queries.sequence(select);
                let query = match query {
                    Ok(query) => query,
                    Err(_e) => {
                        println!("param: select xpath parse failed: {}", select);
                        continue;
                    }
                };
                let dynamic_context_builder = query.dynamic_context_builder(&documents);
                let dynamic_context = dynamic_context_builder.build();
                let result = query.execute_with_context(&mut documents, &dynamic_context);
                let result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return TestOutcome::EnvironmentError(format!(
                            "Error reading stylesheet: {}",
                            error
                        ))
                    }
                };
                params.push((name.into(), result));
            }
        }

        let params = Map::new(params).expect("Param names should be unique");

        let mut variables = Variables::new();

        variables.insert(
            Name::name("params"),
            Item::Function(Function::Map(params)).into(),
        );

        let mut static_context_builder = StaticContextBuilder::default();

        static_context_builder.variable_names(variables.keys().cloned());

        let static_context = static_context_builder.build();
        let program = xee_xslt_compiler::parse(static_context, &xslt);
        let program = match program {
            Ok(program) => program,
            Err(error) => {
                return TestOutcome::EnvironmentError(format!(
                    "Error parsing stylesheet: {}",
                    error
                ))
            }
        };

        // let root = run_context.documents.xot().parse(xml).unwrap();

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
        builder.variables(variables);
        builder.current_datetime(chrono::offset::Utc::now().into());
        let context = builder.build();
        let runnable = program.runnable(&context);
        let result = runnable.many(run_context.documents.xot_mut());

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
        let name_query = queries.option("@name/string()", convert_string)?;
        let select_query = queries.option("@select/string()", convert_string)?;
        let params_query = queries.many("param", move |params, item| {
            let name = name_query.execute(params, item)?;
            let select = select_query.execute(params, item)?;
            Ok(Param { name, select })
        })?;

        let xslt_test_query = queries.one("test", move |documents, item| {
            // the base dir is the same as the test set path, but
            // without the filename
            let base_dir = context.path.parent().unwrap();

            let stylesheets = stylesheets_query.execute(documents, item)?;
            let params = params_query.execute(documents, item)?;
            Ok(XsltTest {
                stylesheets,
                params,
                base_dir: base_dir.to_path_buf(),
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
