use std::path::Path;

use anyhow::Result;

use xee_xpath::{context, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::{hashmap::FxIndexSet, ns::XPATH_TEST_NS};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DependencySpec {
    pub(crate) type_: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dependency {
    pub(crate) spec: DependencySpec,
    pub(crate) satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dependencies {
    pub(crate) dependencies: Vec<Dependency>,
}

impl Dependencies {
    pub(crate) fn new(dependencies: Vec<Dependency>) -> Self {
        Self { dependencies }
    }
}
#[derive(Debug)]
pub(crate) struct KnownDependencies {
    specs: FxIndexSet<DependencySpec>,
}

pub(crate) fn xpath_known_dependencies() -> KnownDependencies {
    let specs = vec![
        DependencySpec {
            type_: "spec".to_string(),
            value: "XP20+".to_string(),
        },
        DependencySpec {
            type_: "spec".to_string(),
            value: "XP30+".to_string(),
        },
        DependencySpec {
            type_: "spec".to_string(),
            value: "XP31+".to_string(),
        },
        DependencySpec {
            type_: "feature".to_string(),
            value: "higherOrderFunctions".to_string(),
        },
        DependencySpec {
            type_: "xml-version".to_string(),
            value: "1.0".to_string(),
        },
        DependencySpec {
            type_: "xsd-version".to_string(),
            value: "1.1".to_string(),
        },
    ];
    KnownDependencies::new(&specs)
}

impl KnownDependencies {
    fn new(specs: &[DependencySpec]) -> Self {
        let specs = specs.iter().cloned().collect();
        Self { specs }
    }

    fn is_supported(&self, dependency: &Dependency) -> bool {
        let contains = self.specs.contains(&dependency.spec);
        if dependency.satisfied {
            contains
        } else {
            !contains
        }
    }
}

impl Dependency {
    pub(crate) fn load<'a>(queries: &'a Queries) -> Result<impl Query<Vec<Vec<Dependency>>> + 'a> {
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
                .map(|value| Dependency {
                    spec: DependencySpec {
                        type_: type_.clone(),
                        value: value.to_string(),
                    },
                    satisfied,
                })
                .collect::<Vec<Dependency>>())
        })?;
        Ok(dependency_query)
    }
}

impl Dependencies {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }
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

    // the XML version is supported if the xml-version is the same
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

    // the xsd version is supported if the the xsd-version is the same
    pub(crate) fn is_xsd_version_supported(&self, known_dependencies: &KnownDependencies) -> bool {
        for dependency in &self.dependencies {
            if dependency.spec.type_ == "xsd-version"
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
        if !self.is_xsd_version_supported(known_dependencies) {
            return false;
        }
        self.is_feature_supported(known_dependencies)
    }
}

impl ContextLoadable<Path> for Dependencies {
    fn static_context_builder<'n>(path: &Path) -> context::StaticContextBuilder<'n> {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(XPATH_TEST_NS);
        builder
    }

    fn load_with_context(queries: &Queries, path: &Path) -> Result<impl Query<Self>> {
        let dependency_query = Dependency::load(queries)?;

        Ok(dependency_query.map(|dependencies, _, _| {
            Ok(Dependencies {
                dependencies: dependencies.into_iter().flatten().collect(),
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    use crate::ns::XPATH_TEST_NS;

    #[test]
    fn test_load_dependencies() {
        let xml = format!(
            r#"
<doc xmlns="{}">
  <dependency type="feature" value="non_unicode_codepoint_collation"/>
  <dependency type="spec" value="XP31 XQ31"/>
</doc>"#,
            XPATH_TEST_NS
        );

        let dependencies = Dependencies::load_from_xml_with_context(&xml, &PathBuf::new()).unwrap();

        assert_eq!(
            dependencies,
            Dependencies {
                dependencies: vec![
                    Dependency {
                        spec: DependencySpec {
                            type_: "feature".to_string(),
                            value: "non_unicode_codepoint_collation".to_string(),
                        },
                        satisfied: true,
                    },
                    Dependency {
                        spec: DependencySpec {
                            type_: "spec".to_string(),
                            value: "XP31".to_string(),
                        },
                        satisfied: true,
                    },
                    Dependency {
                        spec: DependencySpec {
                            type_: "spec".to_string(),
                            value: "XQ31".to_string(),
                        },
                        satisfied: true,
                    },
                ],
            }
        );
    }
}
