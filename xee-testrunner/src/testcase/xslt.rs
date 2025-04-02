use anyhow::Result;
use iri_string::types::{IriAbsoluteString, IriString};

use xee_xpath::{context, Queries, Query};
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
    pub(crate) stylesheets: Vec<Stylesheet>,
}

#[derive(Debug)]
pub(crate) struct Stylesheet {
    pub(crate) file: Option<String>,
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
        dbg!(self);
        TestOutcome::Unsupported
    }

    fn load(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        XsltTestCase::load_with_context(queries, context)
    }
}

impl ContextLoadable<LoadContext> for XsltTestCase {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        let file_query = queries.option("@file/string()", convert_string)?;
        let stylesheets_query = queries.many("stylesheet", move |documents, item| {
            let file = file_query.execute(documents, item)?;
            Ok(Stylesheet { file })
        })?;

        let xslt_test_query = queries.one("test", move |documents, item| {
            let stylesheets = stylesheets_query.execute(documents, item)?;
            Ok(XsltTest { stylesheets })
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
