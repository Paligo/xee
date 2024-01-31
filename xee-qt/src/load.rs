use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use xee_xpath::{
    context::DynamicContext, context::StaticContext, sequence::Item, xml::Node, Name, Namespaces,
    Queries, Query, Recurse, Session,
};
use xot::Xot;

use crate::assert;
use crate::environment::SharedEnvironments;
use crate::error::Result;
use crate::qt;

const NS: &str = "http://www.w3.org/2010/09/qt-fots-catalog";

impl qt::TestSet {
    pub(crate) fn load_from_file(xot: Xot, path: &Path) -> Result<Self> {
        let xml_file = File::open(path)?;
        let mut buf_reader = BufReader::new(xml_file);
        let mut xml = String::new();
        buf_reader.read_to_string(&mut xml)?;
        Self::load_from_xml(xot, path, &xml)
    }

    pub(crate) fn load_from_xml(mut xot: Xot, path: &Path, xml: &str) -> Result<Self> {
        let xot_root = xot.parse(xml)?;
        let root = Node::Xot(xot_root);
        let namespaces = Namespaces::new(
            Namespaces::default_namespaces(),
            Some(NS),
            Some(Namespaces::FN_NAMESPACE),
        );

        let static_context = StaticContext::from_namespaces(namespaces);
        let r = {
            let queries = Queries::new(&static_context);

            let (queries, query) = test_set_query(&xot, path, queries)?;

            let dynamic_context = DynamicContext::empty(xot, &static_context);
            let session = queries.session(&dynamic_context);
            // the query has a lifetime for the dynamic context, and a lifetime
            // for the static context
            query.execute(&session, &Item::from(root))?
        };
        xot.remove(xot_root).unwrap();
        Ok(r)
    }
}

impl qt::Catalog {
    // XXX some duplication here with qt::TestSet
    pub(crate) fn load_from_file(path: &Path) -> Result<Self> {
        let xml_file = File::open(path)?;
        let mut buf_reader = BufReader::new(xml_file);
        let mut xml = String::new();
        buf_reader.read_to_string(&mut xml)?;
        Self::load_from_xml(path, &xml)
    }

    pub(crate) fn load_from_xml(path: &Path, xml: &str) -> Result<Self> {
        let mut xot = Xot::new();
        let xot_root = xot.parse(xml)?;
        let root = Node::Xot(xot_root);
        let namespaces = Namespaces::new(
            Namespaces::default_namespaces(),
            Some(NS),
            Some(Namespaces::FN_NAMESPACE),
        );

        let static_context = StaticContext::from_namespaces(namespaces);
        let dynamic_context = DynamicContext::empty(xot, &static_context);
        let r = {
            let queries = Queries::new(&static_context);

            let (queries, query) = catalog_query(dynamic_context.xot(), path, queries)?;

            let session = queries.session(&dynamic_context);
            query.execute(&session, &Item::from(root))?
        };
        xot.remove(xot_root).unwrap();
        Ok(r)
    }
}

fn test_set_query<'a>(
    xot: &'a Xot,
    path: &'a Path,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<qt::TestSet> + 'a)> {
    let name_query = queries.one("@name/string()", convert_string)?;
    let descriptions_query = queries.many("description/string()", convert_string)?;

    let (queries, shared_environments_query) = shared_environments_query(xot, path, queries)?;
    let (queries, dependency_query) = dependency_query(xot, queries)?;
    let (mut queries, test_cases_query) = test_cases_query(xot, path, queries)?;
    let test_set_query = queries.one("/test-set", move |session, item| {
        let name = name_query.execute(session, item)?;
        let descriptions = descriptions_query.execute(session, item)?;
        let dependencies = dependency_query.execute(session, item)?;
        let shared_environments = shared_environments_query.execute(session, item)?;
        let test_cases = test_cases_query.execute(session, item)?;
        Ok(qt::TestSet {
            full_path: path.to_path_buf(),
            name,
            descriptions,
            dependencies: qt::Dependencies::new(dependencies.into_iter().flatten().collect()),
            shared_environments,
            test_cases,
        })
    })?;
    Ok((queries, test_set_query))
}

fn convert_string(_: &Session, item: &Item) -> xee_xpath::error::Result<String> {
    item.to_atomic()?.try_into()
}

fn convert_boolean(session: &Session, item: &Item) -> xee_xpath::error::Result<bool> {
    Ok(convert_string(session, item)? == "true")
}

fn metadata_query<'a>(
    _xot: &'a Xot,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<qt::Metadata> + 'a)> {
    let description_query = queries.option("description/string()", convert_string)?;
    let by_query = queries.one("@by/string()", convert_string)?;
    let on_query = queries.one("@on/string()", convert_string)?;
    let by_query2 = by_query.clone();
    let on_query2 = on_query.clone();
    let created_query = queries.option("created", move |session, item| {
        {
            {
                Ok(qt::Attribution {
                    by: by_query.execute(session, item)?,
                    on: on_query.execute(session, item)?,
                })
            }
        }
    })?;

    let change_query = queries.option("@change/string()", convert_string)?;
    let modified_query = queries.many("modified", move |session, item| {
        let attribution = qt::Attribution {
            by: by_query2.execute(session, item)?,
            on: on_query2.execute(session, item)?,
        };
        let description = change_query.execute(session, item)?;
        Ok(qt::Modification {
            attribution,
            description: description.unwrap_or("".to_string()),
        })
    })?;

    let metadata_query = queries.one(".", move |session, item| {
        let description = description_query.execute(session, item)?;
        let created = created_query.execute(session, item)?;
        let modified = modified_query.execute(session, item)?;
        Ok(qt::Metadata {
            description,
            created,
            modified,
        })
    })?;

    Ok((queries, metadata_query))
}

fn dependency_query<'a>(
    _xot: &'a Xot,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<Vec<Vec<qt::Dependency>>> + 'a)> {
    let satisfied_query = queries.option("@satisfied/string()", convert_string)?;
    let type_query = queries.one("@type/string()", convert_string)?;
    let value_query = queries.one("@value/string()", convert_string)?;

    let dependency_query = queries.many("dependency", move |session, item| {
        let satisfied = satisfied_query.execute(session, item)?;
        let satisfied = if let Some(satisfied) = satisfied {
            if satisfied == "true" {
                true
            } else if satisfied == "false" {
                false
            } else {
                panic!("Unexpected satisfied value: {:?}", satisfied);
            }
        } else {
            true
        };
        let value = value_query.execute(session, item)?;
        let values = value.split(' ');
        let type_ = type_query.execute(session, item)?;
        Ok(values
            .map(|value| qt::Dependency {
                spec: qt::DependencySpec {
                    type_: type_.clone(),
                    value: value.to_string(),
                },
                satisfied,
            })
            .collect::<Vec<qt::Dependency>>())
    })?;
    Ok((queries, dependency_query))
}

fn test_cases_query<'a>(
    xot: &'a Xot,
    path: &'a Path,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<Vec<qt::TestCase>> + 'a)> {
    let name_query = queries.one("@name/string()", convert_string)?;
    let (mut queries, metadata_query) = metadata_query(xot, queries)?;
    let test_query = queries.one("test/string()", convert_string)?;

    let ref_query = queries.option("@ref/string()", convert_string)?;
    let (mut queries, environment_query) = environment_spec_query(xot, path, queries)?;
    let local_environment_query = queries.many("environment", move |session, item| {
        let ref_ = ref_query.execute(session, item)?;
        if let Some(ref_) = ref_ {
            Ok(qt::TestCaseEnvironment::Ref(qt::EnvironmentRef { ref_ }))
        } else {
            Ok(qt::TestCaseEnvironment::Local(Box::new(
                environment_query.execute(session, item)?,
            )))
        }
    })?;

    let code_query = queries.one("@code/string()", convert_string)?;
    let error_query = queries.one(".", move |session, item| {
        Ok(qt::TestCaseResult::AssertError(assert::AssertError::new(
            code_query.execute(session, item)?,
        )))
    })?;
    let assert_count_query = queries.one("string()", |_, item| {
        let count: String = item.to_atomic()?.try_into()?;
        // XXX unwrap is a hack
        let count = count.parse::<usize>().unwrap();
        Ok(qt::TestCaseResult::AssertCount(assert::AssertCount::new(
            count,
        )))
    })?;

    let assert_xml_query = queries.one("string()", |_, item| {
        let xml: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::AssertXml(assert::AssertXml::new(xml)))
    })?;

    let assert_eq_query = queries.one("string()", |_, item| {
        let eq: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::AssertEq(assert::AssertEq::new(
            qt::XPathExpr(eq),
        )))
    })?;

    let assert_deep_eq_query = queries.one("string()", |_, item| {
        let eq: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::AssertDeepEq(assert::AssertDeepEq::new(
            qt::XPathExpr(eq),
        )))
    })?;

    let string_value_contents = queries.one("string()", convert_string)?;
    let normalize_space_query = queries.option("@normalize-space/string()", convert_boolean)?;

    let assert_string_value_query = queries.one(".", move |session, item| {
        let string_value = string_value_contents.execute(session, item)?;
        let normalize_space = normalize_space_query
            .execute(session, item)?
            .unwrap_or(false);
        Ok(qt::TestCaseResult::AssertStringValue(
            assert::AssertStringValue::new(string_value, normalize_space),
        ))
    })?;

    let assert_type_query = queries.one("string()", |_, item| {
        let string_value: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::AssertType(assert::AssertType::new(
            string_value,
        )))
    })?;

    let assert_query = queries.one("string()", |_, item| {
        let xpath: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::Assert(assert::Assert::new(
            qt::XPathExpr(xpath),
        )))
    })?;

    let assert_permutation_query = queries.one("string()", |_, item| {
        let xpath: String = item.to_atomic()?.try_into()?;
        Ok(qt::TestCaseResult::AssertPermutation(
            assert::AssertPermutation::new(qt::XPathExpr(xpath)),
        ))
    })?;

    let any_all_recurse = queries.many_recurse("*")?;
    let not_recurse = queries.one_recurse("*")?;

    // we use a local-name query here as it's the easiest way support this:
    // there is a single entry in the "result" element, but this may be
    // "any-of" and this contains a list of entries Using a relative path with
    // `query.option()` to detect entries (like "error", "assert-true", etc)
    // doesn't work for "any-of", as it contains a list of entries.
    let local_name_query = queries.one("local-name()", convert_string)?;
    let result_query = queries.one("result/*", move |session: &Session, item: &Item| {
        let f = |session: &Session, item: &Item, recurse: &Recurse<qt::TestCaseResult>| {
            let local_name = local_name_query.execute(session, item)?;
            let r = if local_name == "any-of" {
                let contents = any_all_recurse.execute(session, item, recurse)?;
                qt::TestCaseResult::AnyOf(assert::AssertAnyOf::new(contents))
            } else if local_name == "all-of" {
                let contents = any_all_recurse.execute(session, item, recurse)?;
                qt::TestCaseResult::AllOf(assert::AssertAllOf::new(contents))
            } else if local_name == "not" {
                let contents = not_recurse.execute(session, item, recurse)?;
                qt::TestCaseResult::Not(assert::AssertNot::new(contents))
            } else if local_name == "error" {
                error_query.execute(session, item)?
            } else if local_name == "assert-true" {
                qt::TestCaseResult::AssertTrue(assert::AssertTrue::new())
            } else if local_name == "assert-false" {
                qt::TestCaseResult::AssertFalse(assert::AssertFalse::new())
            } else if local_name == "assert-count" {
                assert_count_query.execute(session, item)?
            } else if local_name == "assert-xml" {
                assert_xml_query.execute(session, item)?
            } else if local_name == "assert-eq" {
                assert_eq_query.execute(session, item)?
            } else if local_name == "assert-deep-eq" {
                assert_deep_eq_query.execute(session, item)?
            } else if local_name == "assert-string-value" {
                assert_string_value_query.execute(session, item)?
            } else if local_name == "assert" {
                assert_query.execute(session, item)?
            } else if local_name == "assert-permutation" {
                assert_permutation_query.execute(session, item)?
            } else if local_name == "assert-empty" {
                qt::TestCaseResult::AssertEmpty(assert::AssertEmpty::new())
            } else if local_name == "assert-type" {
                assert_type_query.execute(session, item)?
            } else {
                qt::TestCaseResult::Unsupported
                // qt::TestCaseResult::AssertFalse
                // panic!("unknown assertion: {}", local_name);
            };
            Ok(r)
        };
        let recurse = Recurse::new(&f);
        recurse.execute(session, item)
    })?;

    let (mut queries, dependency_query) = dependency_query(xot, queries)?;

    let test_query = queries.many("test-case", move |session, item| {
        Ok(qt::TestCase {
            name: name_query.execute(session, item)?,
            metadata: metadata_query.execute(session, item)?,
            environments: local_environment_query.execute(session, item)?,
            dependencies: qt::Dependencies::new(
                dependency_query
                    .execute(session, item)?
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            modules: Vec::new(),
            test: test_query.execute(session, item)?,
            result: result_query.execute(session, item)?,
        })
    })?;

    Ok((queries, test_query))
}

fn environment_spec_query<'a>(
    xot: &'a Xot,
    path: &'a Path,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<qt::EnvironmentSpec> + 'a)> {
    let file_query = queries.one("@file/string()", convert_string)?;
    let role_query = queries.option("@role/string()", convert_string)?;
    let uri_query = queries.option("@uri/string()", convert_string)?;
    let (mut queries, metadata_query) = metadata_query(xot, queries)?;

    let prefix_query = queries.one("@prefix/string()", convert_string)?;
    let namespace_uri_query = queries.one("@uri/string()", convert_string)?;

    let namespaces_query = queries.many("namespace", move |session, item| {
        let prefix = prefix_query.execute(session, item)?;
        let uri = namespace_uri_query.execute(session, item)?;
        Ok(qt::Namespace { prefix, uri })
    })?;

    let sources_query = queries.many("source", move |session, item| {
        let file = PathBuf::from(file_query.execute(session, item)?);
        let role = role_query.execute(session, item)?;
        let uri = uri_query.execute(session, item)?;
        let metadata = metadata_query.execute(session, item)?;
        // we can return multiple sources if both role and uri are set
        // we flatten it later
        let mut sources = Vec::new();
        if let Some(role) = role {
            if role == "." {
                sources.push(qt::Source {
                    metadata: metadata.clone(),
                    role: qt::SourceRole::Context,
                    file: file.clone(),
                })
            } else {
                sources.push(qt::Source {
                    metadata: metadata.clone(),
                    role: qt::SourceRole::Var(role),
                    file: file.clone(),
                });
            }
        };

        if let Some(uri) = uri {
            sources.push(qt::Source {
                metadata,
                role: qt::SourceRole::Doc(uri),
                file,
            });
        }

        Ok(sources)
    })?;

    let name_query = queries.one("@name/string()", convert_string)?;
    let select_query = queries.option("@select/string()", convert_string)?;
    let as_query = queries.option("@as/string()", convert_string)?;
    let source_query = queries.option("@source/string()", convert_string)?;
    let declared_query = queries.option("@declared/string()", convert_string)?;

    let params_query = queries.many("param", move |session, item| {
        let name = name_query.execute(session, item)?;
        let select = select_query.execute(session, item)?;
        let as_ = as_query.execute(session, item)?;
        let source = source_query.execute(session, item)?;
        let declared = declared_query.execute(session, item)?;

        let declared = declared.map(|declared| declared == "true").unwrap_or(false);

        // TODO: do not handle prefixes yet
        let name = Name::unprefixed(&name);

        Ok(qt::Param {
            name,
            select,
            as_,
            source,
            declared,
        })
    })?;

    // the environment base_dir is the same as the catalog/test set path,
    // but without the file name
    let path = path.parent().unwrap();
    let environment_query = queries.one(".", move |session, item| {
        let sources = sources_query.execute(session, item)?;
        // we need to flatten sources
        let sources = sources.into_iter().flatten().collect::<Vec<qt::Source>>();
        let params = params_query.execute(session, item)?;
        let namespaces = namespaces_query.execute(session, item)?;
        let environment_spec = qt::EnvironmentSpec {
            base_dir: path.to_path_buf(),
            sources,
            params,
            namespaces,
            ..Default::default()
        };

        Ok(environment_spec)
    })?;

    Ok((queries, environment_query))
}

fn shared_environments_query<'a>(
    xot: &'a Xot,
    path: &'a Path,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<SharedEnvironments> + 'a)> {
    let name_query = queries.one("@name/string()", convert_string)?;
    let (mut queries, environment_spec_query) = environment_spec_query(xot, path, queries)?;
    let environments_query = queries.many("environment", move |session, item| {
        let name = name_query.execute(session, item)?;
        let environment_spec = environment_spec_query.execute(session, item)?;
        Ok((name, environment_spec))
    })?;
    let shared_environments_query = queries.one(".", move |session, item| {
        let environments = environments_query.execute(session, item)?;
        Ok(SharedEnvironments::new(environments.into_iter().collect()))
    })?;
    Ok((queries, shared_environments_query))
}

fn catalog_query<'a>(
    xot: &'a Xot,
    path: &'a Path,
    mut queries: Queries<'a>,
) -> Result<(Queries<'a>, impl Query<qt::Catalog> + 'a)> {
    let test_suite_query = queries.one("@test-suite/string()", convert_string)?;
    let version_query = queries.one("@version/string()", convert_string)?;

    let (mut queries, shared_environments_query) = shared_environments_query(xot, path, queries)?;

    let test_set_name_query = queries.one("@name/string()", convert_string)?;
    let test_set_file_query = queries.one("@file/string()", convert_string)?;
    let test_set_query = queries.many("test-set", move |session, item| {
        let name = test_set_name_query.execute(session, item)?;
        let file = PathBuf::from(test_set_file_query.execute(session, item)?);
        Ok(qt::TestSetRef { name, file })
    })?;
    let catalog_query = queries.one("catalog", move |session, item| {
        let test_suite = test_suite_query.execute(session, item)?;
        let version = version_query.execute(session, item)?;
        let shared_environments = shared_environments_query.execute(session, item)?;
        let test_sets = test_set_query.execute(session, item)?;
        let file_paths = test_sets.iter().map(|t| t.file.clone()).collect();
        Ok(qt::Catalog {
            full_path: path.to_path_buf(),
            test_suite,
            version,
            shared_environments,
            test_sets,
            file_paths,
        })
    })?;
    Ok((queries, catalog_query))
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_debug_snapshot;

    const ROOT_FIXTURE: &str = include_str!("fixtures/root.xml");
    const PARAMS_FIXTURE: &str = include_str!("fixtures/params.xml");
    const CATALOG_FIXTURE: &str = include_str!("fixtures/catalog.xml");

    #[test]
    fn test_load() {
        let mut xot = Xot::new();
        assert_debug_snapshot!(qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("qt/fn/test.xml"),
            ROOT_FIXTURE
        )
        .unwrap());
    }

    #[test]
    fn test_load_params() {
        let mut xot = Xot::new();
        assert_debug_snapshot!(qt::TestSet::load_from_xml(
            xot,
            &PathBuf::from("qt/fn/test.xml"),
            PARAMS_FIXTURE
        )
        .unwrap());
    }

    #[test]
    fn test_load_catalog() {
        assert_debug_snapshot!(qt::Catalog::load_from_xml(
            &PathBuf::from("qt/catalog.xml"),
            CATALOG_FIXTURE
        )
        .unwrap());
    }
}
