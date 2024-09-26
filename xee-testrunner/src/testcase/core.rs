use std::path::Path;

use xee_name::Namespaces;
// use anyhow::Result;
use xee_xpath::{Queries, Query};
use xee_xpath_compiler::{context::Variables, sequence};
use xee_xpath_load::{convert_string, ContextLoadable, Loadable};

use crate::{
    catalog::Catalog,
    dependency::{Dependencies, Dependency},
    environment::{Environment, EnvironmentIterator, EnvironmentRef, TestCaseEnvironment},
    metadata::Metadata,
    ns::{namespaces, XPATH_TEST_NS},
    runcontext::RunContext,
    testset::TestSet,
};

use super::{assert::TestCaseResult, outcome::TestOutcome};

pub(crate) trait Runnable<E: Environment>: std::marker::Sized {
    fn test_case(&self) -> &TestCase<E>;

    fn run(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<E, Self>,
        test_set: &TestSet<E, Self>,
    ) -> TestOutcome;

    fn load<'a>(
        queries: Queries<'a>,
        path: &'a Path,
    ) -> anyhow::Result<(Queries<'a>, impl Query<Self> + 'a)>
    where
        E: 'a;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TestCase<E: Environment> {
    pub(crate) name: String,
    pub(crate) metadata: Metadata,
    // environments can be a reference by name, or a locally defined environment
    pub(crate) environments: Vec<TestCaseEnvironment<E>>,
    pub(crate) dependencies: Dependencies,
    pub(crate) result: TestCaseResult,
    // pub(crate) modules: Vec<Module>,
}

impl<E: Environment> TestCase<E> {
    pub(crate) fn environments<'a, R: Runnable<E>>(
        &'a self,
        catalog: &'a Catalog<E, R>,
        test_set: &'a TestSet<E, R>,
    ) -> EnvironmentIterator<'a, E> {
        EnvironmentIterator::new(
            vec![&catalog.shared_environments, &test_set.shared_environments],
            &self.environments,
        )
    }

    pub(crate) fn context_item<R: Runnable<E>>(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<E, R>,
        test_set: &TestSet<E, R>,
    ) -> anyhow::Result<Option<sequence::Item>> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        for environment in environments {
            let item = environment
                .environment_spec()
                .context_item(&mut run_context.session)?;
            if let Some(item) = item {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    pub(crate) fn variables<R: Runnable<E>>(
        &self,
        run_context: &mut RunContext,
        catalog: &Catalog<E, R>,
        test_set: &TestSet<E, R>,
    ) -> anyhow::Result<Variables> {
        let environments = self
            .environments(catalog, test_set)
            .collect::<std::result::Result<Vec<_>, crate::error::Error>>()?;
        let mut variables = Variables::new();
        for environment in environments {
            variables.extend(
                environment
                    .environment_spec()
                    .variables(&mut run_context.session)?,
            );
        }
        Ok(variables)
    }
}

impl<E: Environment> ContextLoadable<Path> for TestCase<E> {
    fn xpath_namespaces<'n>() -> Namespaces<'n> {
        namespaces(XPATH_TEST_NS)
    }

    fn load_with_context<'a>(
        mut queries: Queries<'a>,
        path: &'a Path,
    ) -> anyhow::Result<(Queries<'a>, impl Query<Self> + 'a)>
    where
        E: 'a,
    {
        let name_query = queries.one("@name/string()", convert_string)?;
        let (mut queries, metadata_query) = Metadata::load(queries)?;

        let ref_query = queries.option("@ref/string()", convert_string)?;
        let (mut queries, environment_query) = E::load(queries, path)?;
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

        let (queries, result_query) = TestCaseResult::load(queries)?;
        let (mut queries, dependency_query) = Dependency::load(queries)?;
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

        Ok((queries, test_case_query))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        environment::XPathEnvironmentSpec, metadata::Attribution, ns::XPATH_TEST_NS,
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

        let test_case =
            TestCase::<XPathEnvironmentSpec>::load_from_xml_with_context(&xml, &path).unwrap();
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
