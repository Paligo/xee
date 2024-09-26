use anyhow::Result;
use std::{
    cell::RefCell,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
};
use xee_name::Namespaces;
use xee_xpath::{Queries, Query, Session};
use xee_xpath_compiler::{
    context::{DynamicContext, StaticContext, Variables},
    parse, sequence,
    xml::Documents,
    Name,
};
use xee_xpath_load::{convert_string, ContextLoadable};
use xot::Xot;

use crate::ns::{namespaces, XPATH_TEST_NS};

use super::{
    collation::Collation,
    collection::Collection,
    resource::Resource,
    source::{Source, SourceRole},
};

// the abstract environment. Can be an XPath or XSLT environment.
pub(crate) trait Environment: Sized {
    // create an empty environment
    fn empty() -> Self;

    // get the underlying environment spec
    fn environment_spec(&self) -> &EnvironmentSpec;

    // a query to load it from XML
    fn load<'a>(
        queries: Queries<'a>,
        path: &'a Path,
    ) -> Result<(Queries<'a>, impl Query<Self> + 'a)>;
}

// In a test case we can include an environment directly, or refer to an environment
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TestCaseEnvironment<E: Environment> {
    Local(Box<E>),
    Ref(EnvironmentRef),
}

// a way to reference to other environments
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentRef {
    pub(crate) ref_: String,
}

impl EnvironmentRef {
    pub(crate) fn new(ref_: String) -> Self {
        Self { ref_ }
    }
}

impl Display for EnvironmentRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ref_)
    }
}

// environment information shared by XPath and XSLT
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct EnvironmentSpec {
    pub(crate) base_dir: PathBuf,

    pub(crate) sources: Vec<Source>,
    pub(crate) params: Vec<Param>,
    // TODO
    pub(crate) collations: Vec<Collation>,
    // TODO: needs to wait until the interpreter has a resource abstraction
    pub(crate) resources: Vec<Resource>,
    // TODO: needs to wait until the interpreter has a collection abstraction
    pub(crate) collections: Vec<Collection>,
    // not supported as Xee doesn't support XML schema
    pub(crate) schemas: Vec<Schema>,
    // Not in use at all?
    // pub(crate) function_libraries: Vec<FunctionLibrary>,
}

// Not supported yet: schema support not implemented in Xee
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Schema {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Param {
    pub(crate) name: Name,
    pub(crate) select: Option<String>,
    // TODO: not supported yet
    pub(crate) as_: Option<String>,
    // TODO: not supported yet
    pub(crate) static_: bool,
    // XQuery related, not supported
    pub(crate) declared: bool,
    // Doesn't appear to be in use, even though it's in the schema
    pub(crate) source: Option<String>,
}

impl EnvironmentSpec {
    pub(crate) fn empty() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub(crate) fn context_item(&self, session: &mut Session) -> Result<Option<sequence::Item>> {
        for source in &self.sources {
            if let SourceRole::Context = source.role {
                let node = source.node(&self.base_dir, session)?;
                return Ok(Some(sequence::Item::from(node)));
            }
        }
        Ok(None)
    }

    pub(crate) fn variables(&self, session: &mut Session) -> Result<Variables> {
        let mut variables = Variables::new();
        for source in &self.sources {
            if let SourceRole::Var(name) = &source.role {
                let name = &name[1..]; // without $
                let node = source.node(&self.base_dir, session)?;
                variables.insert(Name::name(name), sequence::Item::from(node).into());
            }
        }
        for param in &self.params {
            let static_context = StaticContext::default();
            let documents = RefCell::new(Documents::new());
            let select = (param.select.as_ref()).expect("param: missing select not supported");
            let program = parse(&static_context, select);
            if program.is_err() {
                println!("param: select xpath parse failed: {}", select);
                continue;
            }
            let program = program.unwrap();
            let dynamic_context = DynamicContext::from_documents(&static_context, &documents);
            let runnable = program.runnable(&dynamic_context);
            let result = runnable
                .many(None, xot, Variables::new())
                .map_err(|e| e.error)?;
            variables.insert(param.name.clone(), result);
        }
        Ok(variables)
    }
}

impl ContextLoadable<Path> for EnvironmentSpec {
    fn xpath_namespaces<'n>() -> Namespaces<'n> {
        namespaces(XPATH_TEST_NS)
    }

    fn load_with_context<'a>(
        queries: Queries<'a>,
        path: &'a Path,
    ) -> Result<(Queries<'a>, impl Query<Self> + 'a)>
    where
        EnvironmentSpec: 'a,
    {
        let (mut queries, sources_query) = Source::load(queries)?;

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
            let name = Name::name(&name);

            Ok(Param {
                name,
                select,
                as_,
                source,
                declared,
                // TODO
                static_: false,
            })
        })?;

        // the environment base_dir is the same as the catalog/test set path,
        // but without the file name
        let path = path.parent().unwrap();
        let environment_query = queries.one(".", move |session, item| {
            let sources = sources_query.execute(session, item)?;
            // we need to flatten sources
            let sources = sources.into_iter().flatten().collect::<Vec<Source>>();
            let params = params_query.execute(session, item)?;

            let environment_spec = EnvironmentSpec {
                base_dir: path.to_path_buf(),
                sources,
                params,
                ..Default::default()
            };

            Ok(environment_spec)
        })?;

        Ok((queries, environment_query))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        environment::source::SourceContent,
        metadata::Metadata,
        ns::{namespaces, XPATH_TEST_NS},
    };

    use super::*;

    #[test]
    fn test_load_environment_spec() {
        let xml = format!(
            r#"
            <environment xmlns="{}">
                <source file="a.xml" role="."/>
                <source file="b.xml" role="$var"/>
                <param name="p1" select="'1'"/>
                <param name="p2" select="'2'"/>
            </environment>"#,
            XPATH_TEST_NS
        );

        let path = Path::new("bar/foo");
        let environment_spec = EnvironmentSpec::load_from_xml_with_context(&xml, path).unwrap();
        assert_eq!(
            environment_spec,
            EnvironmentSpec {
                base_dir: PathBuf::from("bar"),
                sources: vec![
                    Source {
                        content: SourceContent::Path(PathBuf::from("a.xml")),
                        role: SourceRole::Context,
                        metadata: Metadata {
                            description: None,
                            created: None,
                            modified: vec![],
                        },
                        uri: None,
                        validation: None,
                    },
                    Source {
                        content: SourceContent::Path(PathBuf::from("b.xml")),
                        role: SourceRole::Var("$var".to_string()),
                        metadata: Metadata {
                            description: None,
                            created: None,
                            modified: vec![],
                        },
                        uri: None,
                        validation: None,
                    },
                ],
                params: vec![
                    Param {
                        name: Name::name("p1"),
                        select: Some("'1'".to_string()),
                        as_: None,
                        static_: false,
                        declared: false,
                        source: None,
                    },
                    Param {
                        name: Name::name("p2"),
                        select: Some("'2'".to_string()),
                        as_: None,
                        static_: false,
                        declared: false,
                        source: None,
                    },
                ],
                ..Default::default()
            }
        )
    }

    #[test]
    fn test_load_environment_spec_with_content() {
        let xml = format!(
            r#"
            <environment xmlns="{}">
                <source role="."><content>Foo</content></source>
            </environment>"#,
            XPATH_TEST_NS
        );

        let path = Path::new("bar/foo");

        let environment_spec = EnvironmentSpec::load_from_xml_with_context(&xml, path).unwrap();
        assert_eq!(
            environment_spec,
            EnvironmentSpec {
                base_dir: PathBuf::from("bar"),
                sources: vec![Source {
                    content: SourceContent::String("Foo".to_string()),
                    role: SourceRole::Context,
                    metadata: Metadata {
                        description: None,
                        created: None,
                        modified: vec![],
                    },
                    uri: None,
                    validation: None,
                },],
                ..Default::default()
            }
        )
    }
}
