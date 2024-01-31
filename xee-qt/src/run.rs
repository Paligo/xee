use derive_builder::Builder;
use std::borrow::Cow;
use std::path::Path;
use xee_xpath::{
    context::DynamicContext, context::StaticContext, parse, sequence::Item, xml::Documents,
    Namespaces, Variables,
};
use xot::Xot;

use crate::collection::FxIndexSet;
use crate::environment::EnvironmentSpecIterator;
use crate::error::Result;
use crate::outcome::TestOutcome;
use crate::qt;

#[derive(Debug)]
pub(crate) struct KnownDependencies {
    specs: FxIndexSet<qt::DependencySpec>,
}

impl KnownDependencies {
    fn new(specs: &[qt::DependencySpec]) -> Self {
        let specs = specs.iter().cloned().collect();
        Self { specs }
    }

    fn is_supported(&self, dependency: &qt::Dependency) -> bool {
        let contains = self.specs.contains(&dependency.spec);
        if dependency.satisfied {
            contains
        } else {
            !contains
        }
    }
}

impl Default for KnownDependencies {
    fn default() -> Self {
        let specs = vec![
            qt::DependencySpec {
                type_: "spec".to_string(),
                value: "XP20+".to_string(),
            },
            qt::DependencySpec {
                type_: "spec".to_string(),
                value: "XP30+".to_string(),
            },
            qt::DependencySpec {
                type_: "spec".to_string(),
                value: "XP31+".to_string(),
            },
            qt::DependencySpec {
                type_: "feature".to_string(),
                value: "higherOrderFunctions".to_string(),
            },
            qt::DependencySpec {
                type_: "xml-version".to_string(),
                value: "1.0".to_string(),
            },
        ];
        Self::new(&specs)
    }
}

// TODO: if an environment with a schema is referenced, then schema-awareness
// is an implicit dependency

#[derive(Builder)]
#[builder(pattern = "owned")]
pub(crate) struct RunContext<'a> {
    pub(crate) dynamic_context: DynamicContext<'a>,
    pub(crate) catalog: qt::Catalog,
    #[builder(default)]
    pub(crate) documents: Documents,
    #[builder(default)]
    pub(crate) known_dependencies: KnownDependencies,
    #[builder(default)]
    pub(crate) verbose: bool,
}

impl<'a> RunContext<'a> {
    pub(crate) fn new(dynamic_context: DynamicContext<'a>, catalog: qt::Catalog) -> Self {
        Self {
            dynamic_context,
            catalog,
            documents: Documents::new(),
            known_dependencies: KnownDependencies::default(),
            verbose: false,
        }
    }

    pub(crate) fn from_path(path: &Path) -> Result<Self> {
        let catalog = qt::Catalog::load_from_file(path)?;

        let mut xot = Xot::new();
        let static_context = StaticContext::default();
        let dynamic_context = DynamicContext::empty(xot, &static_context);
        Ok(RunContextBuilder::default()
            .dynamic_context(dynamic_context)
            .catalog(catalog)
            .verbose(false)
            .build()
            .unwrap())
    }
}

impl<'a> Drop for RunContext<'a> {
    fn drop(&mut self) {
        self.documents.cleanup(self.dynamic_context.xot_mut());
    }
}

impl qt::TestSet {
    fn base_dir(&self) -> &Path {
        self.full_path.parent().unwrap()
    }

    pub(crate) fn file_path(&self, catalog: &qt::Catalog) -> &Path {
        self.full_path.strip_prefix(catalog.base_dir()).unwrap()
    }

    pub(crate) fn test_case_by_name(&self, name: &str) -> Option<&qt::TestCase> {
        self.test_cases
            .iter()
            .find(|test_case| test_case.name == name)
    }
}

impl qt::Catalog {
    pub(crate) fn base_dir(&self) -> &Path {
        self.full_path.parent().unwrap()
    }
}

impl qt::Dependencies {
    // the spec is supported if any of the spec dependencies is supported
    pub(crate) fn is_spec_supported(&self, known_dependencies: &KnownDependencies) -> bool {
        let mut spec_dependency_seen: bool = false;
        for dependency in &self.dependencies {
            if dependency.spec.type_ == "spec" {
                spec_dependency_seen = true;
                if known_dependencies.is_supported(dependency) {
                    return true;
                }
            }
        }
        // if we haven't seen any spec dependencies, then we're supported
        // otherwise, we aren't
        !spec_dependency_seen
    }

    pub(crate) fn is_feature_supported(&self, known_dependencies: &KnownDependencies) -> bool {
        for dependency in &self.dependencies {
            // if a listed feature dependency is not supported, we don't support this
            if dependency.spec.type_ == "feature" && !known_dependencies.is_supported(dependency) {
                return false;
            }
        }
        true
    }

    // the XML version is supported if the the xml-version is the same
    pub(crate) fn is_xml_version_supported(&self, known_dependencies: &KnownDependencies) -> bool {
        for dependency in &self.dependencies {
            if dependency.spec.type_ == "xml-version"
                && !known_dependencies.is_supported(dependency)
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn is_supported(&self, known_dependencies: &KnownDependencies) -> bool {
        // if we have no dependencies, we're always supported
        if self.dependencies.is_empty() {
            return true;
        }
        // if we don't support the spec, we don't support it
        if !self.is_spec_supported(known_dependencies) {
            return false;
        }
        if !self.is_xml_version_supported(known_dependencies) {
            return false;
        }
        self.is_feature_supported(known_dependencies)
    }
}

impl qt::TestCase {
    pub(crate) fn run(&self, mut run_context: RunContext, test_set: &qt::TestSet) -> TestOutcome {
        let variables = self.variables(run_context, test_set);
        let variables = match variables {
            Ok(variables) => variables,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };

        let context_item = self.context_item(&mut run_context, test_set);
        let context_item = match context_item {
            Ok(context_item) => context_item,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };

        let namespaces = self.namespaces(&mut run_context, test_set);
        let namespaces = match namespaces {
            Ok(namespaces) => namespaces,
            Err(error) => return TestOutcome::EnvironmentError(error.to_string()),
        };

        let variable_names = variables.iter().map(|(name, _)| name.clone()).collect();
        let static_context = StaticContext::new(namespaces, variable_names);
        let program = parse(&static_context, &self.test);
        let program = match program {
            Ok(xpath) => xpath,
            Err(error) => {
                return match &self.result {
                    qt::TestCaseResult::AssertError(assert_error) => {
                        assert_error.assert_error(&error.error)
                    }
                    qt::TestCaseResult::AnyOf(any_of) => any_of.assert_error(&error.error),
                    _ => TestOutcome::CompilationError(error.error),
                }
            }
        };

        let dynamic_context = DynamicContext::new(
            run_context.xot,
            &static_context,
            Cow::Borrowed(&run_context.documents),
        );
        let runnable = program.runnable(&dynamic_context);
        let result = runnable.many(context_item.as_ref(), variables);
        self.result
            .assert_result(&runnable, &result.map_err(|error| error.error))
    }

    fn environment_specs<'a>(
        &'a self,
        catalog: &'a qt::Catalog,
        test_set: &'a qt::TestSet,
    ) -> EnvironmentSpecIterator<'a> {
        EnvironmentSpecIterator {
            environments: &self.environments,
            catalog_shared_environments: &catalog.shared_environments,
            test_set_shared_environments: &test_set.shared_environments,
            index: 0,
        }
    }

    fn context_item(
        &self,
        run_context: &mut RunContext,
        test_set: &qt::TestSet,
    ) -> Result<Option<Item>> {
        let environment_specs = self
            .environment_specs(&run_context.catalog, test_set)
            .collect::<Result<Vec<_>>>()?;
        let xot = &mut run_context.xot;
        let documents = &mut run_context.documents;
        for environment_spec in environment_specs {
            let item = environment_spec.context_item(xot, documents)?;
            if let Some(item) = item {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }

    fn variables(&self, mut run_context: RunContext, test_set: &qt::TestSet) -> Result<Variables> {
        let environment_specs = self
            .environment_specs(&run_context.catalog, test_set)
            .collect::<Result<Vec<_>>>()?;
        let mut variables = Variables::new();
        let source_cache = &mut run_context.documents;
        for environment_spec in environment_specs {
            variables.extend(environment_spec.variables(run_context.xot, source_cache)?);
        }
        Ok(variables)
    }

    fn namespaces<'a>(
        &'a self,
        run_context: &'a RunContext,
        test_set: &'a qt::TestSet,
    ) -> Result<Namespaces<'a>> {
        let environment_specs = self
            .environment_specs(&run_context.catalog, test_set)
            .collect::<Result<Vec<_>>>()?;
        let mut namespaces = Namespaces::default();
        for environment_spec in environment_specs {
            namespaces.add(&environment_spec.namespace_pairs())
        }
        Ok(namespaces)
    }
}

#[cfg(test)]
mod tests {
    use ibig::ibig;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    use xee_xpath::{atomic::Atomic, error::Error, sequence::Sequence};

    use crate::assert;
    use crate::assert::{AssertCountFailure, AssertStringValueFailure, Failure};
    use crate::outcome::{TestOutcome, UnexpectedError};

    use super::*;
    const CATALOG_FIXTURE: &str = include_str!("fixtures/catalog.xml");

    fn run(test_set: &qt::TestSet) -> TestOutcome {
        let catalog =
            qt::Catalog::load_from_xml(&PathBuf::from("my/catalog.xml"), CATALOG_FIXTURE).unwrap();

        let mut xot = Xot::new();
        let mut run_context = RunContext::new(xot, catalog);
        assert_eq!(test_set.test_cases.len(), 1);
        let test_case = &test_set.test_cases[0];
        test_case.run(run_context, test_set)
    }

    fn load(xot: Xot, test_cases_path: &Path) -> qt::TestSet {
        qt::TestSet::load_from_file(xot, test_cases_path).unwrap()
    }

    #[test]
    fn test_assert_true() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 eq 1</test>
    <result>
      <assert-true />
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_true_fails() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 ne 1</test>
    <result>
      <assert-true />
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::Failed(Failure::True(
                assert::AssertTrue,
                Sequence::from(vec![Item::from(Atomic::from(false))])
            ))
        );
    }

    #[test]
    fn test_assert_count() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1, 2, 3</test>
    <result>
      <assert-count>3</assert-count>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_expected_error() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 div 0</test>
    <result>
      <error code="FOAR0001"/>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_error_code_unexpected() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 div 0</test>
    <result>
      <error code="FOAR0002"/>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::PassedWithUnexpectedError(UnexpectedError::Code("FOAR0001".to_string()))
        );
    }

    #[test]
    fn test_unexpected_runtime_error() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 div 0</test>
    <result>
      <assert-true/>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::RuntimeError(Error::FOAR0001)
        );
    }

    #[test]
    fn test_unexpected_compilation_error() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 @#!</test>
    <result>
      <assert-true/>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::CompilationError(Error::XPST0003)
        );
    }

    #[test]
    fn test_expected_compilation_error() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>1 @#!</test>
    <result>
      <error code="XPST0003" />
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_eq_passes() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_eq_passes2() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>4 div 2</test>
    <result>
      <assert-eq>2</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_eq_fails() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="true">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <test>5</test>
    <result>
      <assert-eq>6</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::Failed(Failure::Eq(
                assert::AssertEq::new(qt::XPathExpr("6".to_string())),
                Sequence::from(vec![Item::from(Atomic::from(ibig!(5)))])
            ))
        );
    }

    #[test]
    fn test_assert_any_of_error_case() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>1 div 0</test>
        <result>
          <any-of>
            <assert-eq>2</assert-eq>
            <error code="FOAR0001"/>
          </any-of>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_any_of_eq_case() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>4 div 2</test>
        <result>
          <any-of>
            <assert-eq>2</assert-eq>
            <error code="FOAR0001"/>
          </any-of>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_all_of_passes() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>4 div 2</test>
        <result>
          <all-of>
            <assert-eq>2</assert-eq>
            <assert-count>1</assert-count>
          </all-of>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_all_of_fails() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>4 div 2</test>
        <result>
          <all-of>
            <assert-eq>2</assert-eq>
            <assert-count>8</assert-count>
          </all-of>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::Failed(Failure::Count(
                assert::AssertCount::new(8),
                AssertCountFailure::WrongCount(1)
            ))
        );
    }

    #[test]
    fn test_assert_string_value_passes() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>"foo"</test>
        <result>
          <assert-string-value>foo</assert-string-value>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_string_value_fails() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>"foo"</test>
        <result>
          <assert-string-value>foo2</assert-string-value>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(
            run(xot, &test_set),
            TestOutcome::Failed(Failure::StringValue(
                assert::AssertStringValue::new("foo2".to_string(), false),
                AssertStringValueFailure::WrongStringValue("foo".to_string())
            ))
        );
    }

    #[test]
    fn test_assert_string_value_sequence_passes() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#"
    <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
      <test-case name="true">
        <description>Description</description>
        <created by="Martijn Faassen" on="2023-05-22"/>
        <environment ref="empty"/>
        <test>"foo", 3</test>
        <result>
          <assert-string-value>foo 3</assert-string-value>
        </result>
      </test-case>
      </test-set>"#,
        )
        .unwrap();
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_external_environment_source() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <environment name="data">
            <source role="." file="data.xml">
            </source>
          </environment>
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment ref="data"/>
            <test>/doc/p/string()</test>
            <result>
              <assert-string-value>Hello world!</assert-string-value>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(data_file, r#"<doc><p>Hello world!</p></doc>"#).unwrap();
        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_local_environment_source() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment name="data">
              <source role="." file="data.xml">
              </source>
            </environment>
            <test>/doc/p/string()</test>
            <result>
              <assert-string-value>Hello world!</assert-string-value>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(data_file, r#"<doc><p>Hello world!</p></doc>"#).unwrap();
        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_xml_passed() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <environment name="data">
            <source role="." file="data.xml">
            </source>
          </environment>
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment ref="data"/>
            <test>/doc/p</test>
            <result>
              <assert-xml><![CDATA[<p>Hello world!</p>]]></assert-xml>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(data_file, r#"<doc><p>Hello world!</p></doc>"#).unwrap();
        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);
        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_xml_failed() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <environment name="data">
            <source role="." file="data.xml">
            </source>
          </environment>
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment ref="data"/>
            <test>/doc/p</test>
            <result>
              <assert-xml><![CDATA[<p>Something else!</p>]]></assert-xml>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(data_file, r#"<doc><p>Hello world!</p></doc>"#).unwrap();

        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);

        assert_eq!(
            run(xot, &test_set),
            TestOutcome::Failed(Failure::Xml(
                assert::AssertXml::new("<p>Something else!</p>".to_string()),
                assert::AssertXmlFailure::WrongXml(
                    "<sequence><p>Hello world!</p></sequence>".to_string()
                )
            ))
        );
    }

    #[test]
    fn test_assert_xml_sequence() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <environment name="data">
            <source role="." file="data.xml">
            </source>
          </environment>
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment ref="data"/>
            <test>/doc/p</test>
            <result>
              <assert-xml><![CDATA[<p>Hello world!</p><p>Grabthar's Hammer</p>]]></assert-xml>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(
            data_file,
            r#"<doc><p>Hello world!</p><p>Grabthar's Hammer</p></doc>"#
        )
        .unwrap();

        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);

        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_assert_variable_source() {
        let tmp_dir = tempdir().unwrap();
        let test_cases_path = tmp_dir.path().join("test_cases.xml");
        let mut test_cases_file = File::create(&test_cases_path).unwrap();
        write!(
            test_cases_file,
            r#"
        <test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
          <test-case name="true">
            <description>Description</description>
            <created by="Martijn Faassen" on="2023-05-22"/>
            <environment name="data">
              <source role="$data" file="data.xml">
              </source>
            </environment>
            <test>$data/doc/p/string()</test>
            <result>
              <assert-string-value>Hello world!</assert-string-value>
            </result>
          </test-case>
          </test-set>"#,
        )
        .unwrap();

        let mut data_file = File::create(tmp_dir.path().join("data.xml")).unwrap();
        write!(data_file, r#"<doc><p>Hello world!</p></doc>"#).unwrap();

        let mut xot = Xot::new();
        let test_set = load(xot, &test_cases_path);

        assert_eq!(run(xot, &test_set), TestOutcome::Passed);
    }

    #[test]
    fn test_dependency_spec_supported() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="test_case">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <dependency type="spec" value="XP30+" />
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        let test_case = test_set.test_case_by_name("test_case").unwrap();

        let specs = vec![qt::DependencySpec {
            type_: "spec".to_string(),
            value: "XP30+".to_string(),
        }];

        let known_dependencies = KnownDependencies::new(&specs);

        assert!(test_case.dependencies.is_supported(&known_dependencies));
    }

    #[test]
    fn test_dependency_spec_supported2() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="test_case">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <dependency type="spec" value="XP30+ XQ30+" />
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        let test_case = test_set.test_case_by_name("test_case").unwrap();

        let specs = vec![qt::DependencySpec {
            type_: "spec".to_string(),
            value: "XP30+".to_string(),
        }];

        let known_dependencies = KnownDependencies::new(&specs);

        assert!(test_case.dependencies.is_supported(&known_dependencies));
    }

    #[test]
    fn test_dependency_feature_supported() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="test_case">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <dependency type="spec" value="XP30+" />
    <dependency type="feature" value="FOO" />
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        let test_case = test_set.test_case_by_name("test_case").unwrap();

        let specs = vec![qt::DependencySpec {
            type_: "spec".to_string(),
            value: "XP30+".to_string(),
        }];

        let known_dependencies = KnownDependencies::new(&specs);

        assert!(!test_case.dependencies.is_supported(&known_dependencies));
    }

    #[test]
    fn test_dependency_feature_supported2() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="test_case">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <dependency type="spec" value="XP30+" />
    <dependency type="feature" value="FOO" />
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        let test_case = test_set.test_case_by_name("test_case").unwrap();

        let specs = vec![
            qt::DependencySpec {
                type_: "spec".to_string(),
                value: "XP30+".to_string(),
            },
            qt::DependencySpec {
                type_: "feature".to_string(),
                value: "FOO".to_string(),
            },
        ];

        let known_dependencies = KnownDependencies::new(&specs);

        assert!(test_case.dependencies.is_supported(&known_dependencies));
    }

    #[test]
    fn test_dependency_feature_supported3() {
        let mut xot = Xot::new();
        let test_set = qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("my/test.xml"),
            r#" 
<test-set xmlns="http://www.w3.org/2010/09/qt-fots-catalog" name="test">
  <test-case name="test_case">
    <description>Description</description>
    <created by="Martijn Faassen" on="2023-05-22"/>
    <environment ref="empty"/>
    <dependency type="spec" value="XP30+" />
    <dependency type="feature" value="FOO BAR" />
    <test>5</test>
    <result>
      <assert-eq>5</assert-eq>
    </result>
  </test-case>
  </test-set>"#,
        )
        .unwrap();
        let test_case = test_set.test_case_by_name("test_case").unwrap();

        let specs = vec![
            qt::DependencySpec {
                type_: "spec".to_string(),
                value: "XP30+".to_string(),
            },
            qt::DependencySpec {
                type_: "feature".to_string(),
                value: "FOO".to_string(),
            },
        ];

        let known_dependencies = KnownDependencies::new(&specs);

        assert!(!test_case.dependencies.is_supported(&known_dependencies));
    }
}
