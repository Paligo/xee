use ahash::{HashMap, HashMapExt};
use iri_string::types::IriAbsoluteStr;
use xee_xpath::{context, Item, Queries, Query, Sequence};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::{
    catalog::{Catalog, LoadContext},
    dependency::{Dependencies, Dependency},
    environment::{Environment, EnvironmentIterator, EnvironmentRef, TestCaseEnvironment},
    language::Language,
    metadata::Metadata,
    runcontext::RunContext,
    testset::TestSet,
};

use super::{assert::TestCaseResult, outcome::TestOutcome};

pub(crate) trait Runnable<L: Language>: std::marker::Sized {
    fn test_case(&self) -> &TestCase<L>;

    fn run(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
    ) -> TestOutcome;

    fn load(queries: &Queries, context: &LoadContext) -> anyhow::Result<impl Query<Self>>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TestCase<L: Language> {
    pub(crate) name: String,
    pub(crate) metadata: Metadata,
    // environments can be a reference by name, or a locally defined environment
    pub(crate) environments: Vec<TestCaseEnvironment<L::Environment>>,
    pub(crate) dependencies: Dependencies,
    pub(crate) result: TestCaseResult,
    // pub(crate) modules: Vec<Module>,
}

impl<L: Language> TestCase<L> {
    pub(crate) fn environments<'a>(
        &'a self,
        catalog: &'a Catalog<L>,
        test_set: &'a TestSet<L>,
    ) -> EnvironmentIterator<'a, L::Environment> {
        EnvironmentIterator::new(
            vec![&catalog.shared_environments, &test_set.shared_environments],
            &self.environments,
        )
    }

    pub(crate) fn static_base_uri<'a>(
        &'a self,
        catalog: &'a Catalog<L>,
        test_set: &'a TestSet<L>,
    ) -> anyhow::Result<Option<&'a str>> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        for environment in environments {
            let static_base_uri = &environment.environment_spec().static_base_uri;
            if let Some(static_base_uri) = static_base_uri {
                return Ok(Some(static_base_uri));
            }
        }
        Ok(None)
    }

    pub(crate) fn load_sources(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> anyhow::Result<()> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        for environment in environments {
            environment
                .environment_spec()
                .load_sources(run_context.documents, base_uri)?;
        }
        Ok(())
    }

    pub(crate) fn load_collections(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<L>,
        test_test: &TestSet<L>,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> anyhow::Result<HashMap<String, Sequence>> {
        let mut collections = HashMap::new();
        let environments = self
            .environments(catalog, test_test)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        for environment in environments {
            collections.extend(
                environment
                    .environment_spec()
                    .load_collections(run_context.documents, base_uri)?,
            );
        }
        Ok(collections)
    }

    pub(crate) fn context_item(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> anyhow::Result<Option<Item>> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        for environment in environments {
            let item = environment
                .environment_spec()
                .context_item(run_context.documents, base_uri)?;
            if let Some(item) = item {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    pub(crate) fn variables(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<L>,
        test_set: &TestSet<L>,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> anyhow::Result<context::Variables> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        let mut variables = context::Variables::new();
        for environment in environments {
            variables.extend(
                environment
                    .environment_spec()
                    .variables(run_context.documents, base_uri)?,
            );
        }
        Ok(variables)
    }
}

impl<L: Language> ContextLoadable<LoadContext> for TestCase<L> {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(
        queries: &Queries,
        context: &LoadContext,
    ) -> anyhow::Result<impl Query<Self>> {
        let name_query = queries.one("@name/string()", convert_string)?;
        let metadata_query = Metadata::load_with_context(queries, context)?;

        let ref_query = queries.option("@ref/string()", convert_string)?;
        let environment_query = L::Environment::load(queries, context)?;
        let local_environment_query = queries.many("environment", move |session, item| {
            let ref_ = ref_query.execute(session, item)?;
            if let Some(ref_) = ref_ {
                Ok(TestCaseEnvironment::Ref(EnvironmentRef { ref_ }))
            } else {
                Ok(TestCaseEnvironment::Local(Box::new(
                    environment_query.execute(session, item)?,
                )))
            }
        })?;

        let result_query = TestCaseResult::load_with_context(queries, context)?;
        let dependency_query = Dependency::load(queries)?;
        let test_case_query = queries.one(".", move |session, item| {
            let test_case = TestCase {
                name: name_query.execute(session, item)?,
                metadata: metadata_query.execute(session, item)?,
                environments: local_environment_query.execute(session, item)?,
                dependencies: Dependencies::new(
                    dependency_query
                        .execute(session, item)?
                        .into_iter()
                        .flatten()
                        .collect(),
                ),
                result: result_query.execute(session, item)?,
            };
            Ok(test_case)
        })?;

        Ok(test_case_query)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        language::XPathLanguage, metadata::Attribution, ns::XPATH_TEST_NS,
        testcase::assert::AssertTrue,
    };

    use super::*;

    #[test]
    fn test_load_test_case() {
        let xml = format!(
            r#"
<test-case xmlns="{}" name="foo">
  <description>A test case</description>
  <created by="Bar Quxson" on="2024-01-01"/>
  <test>1</test>
  <result>
    <assert-true/>
  </result>
</test-case>"#,
            XPATH_TEST_NS
        );

        let path = PathBuf::from("bar/foo");
        let context = LoadContext {
            path,
            catalog_ns: XPATH_TEST_NS,
        };
        let test_case =
            TestCase::<XPathLanguage>::load_from_xml_with_context(&xml, &context).unwrap();
        assert_eq!(
            test_case,
            TestCase {
                name: "foo".to_string(),
                metadata: Metadata {
                    description: Some("A test case".to_string()),
                    created: Some(Attribution {
                        by: "Bar Quxson".to_string(),
                        on: "2024-01-01".to_string(),
                    }),
                    modified: vec![],
                },
                environments: vec![],
                dependencies: Dependencies::empty(),
                result: TestCaseResult::AssertTrue(AssertTrue),
            }
        )
    }
}
