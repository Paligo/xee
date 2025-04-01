use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use iri_string::types::IriAbsoluteStr;
use xot::xmlname::OwnedName as Name;

use xee_xpath::{context, Documents, Item, Queries, Query, Sequence};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::catalog::LoadContext;

use super::{
    collation::Collation,
    collection::Collection,
    resource::Resource,
    source::{Source, SourceRole, Sources},
};

// the abstract environment. Can be an XPath or XSLT environment.
pub(crate) trait Environment: Sized + std::fmt::Debug {
    // create an empty environment
    fn empty() -> Self;

    // get the underlying environment spec
    fn environment_spec(&self) -> &EnvironmentSpec;

    // a query to load it from XML
    fn load(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>>;
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
    #[cfg(test)]
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
    pub(crate) static_base_uri: Option<String>,

    // TODO
    pub(crate) collations: Vec<Collation>,
    // TODO: needs to wait until the interpreter has a resource abstraction
    pub(crate) resources: Vec<Resource>,
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
    pub(crate) fn load_sources(
        &self,
        documents: &mut Documents,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<()> {
        // load all the sources. since loading a node has a cache,
        // the later context_item load won't clash
        for source in &self.sources {
            let _ = source.node(&self.base_dir, documents, &source.uri, base_uri)?;
        }
        Ok(())
    }

    pub(crate) fn load_collections(
        &self,
        documents: &mut Documents,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<HashMap<String, Sequence>> {
        let mut collections = HashMap::new();
        for collection in &self.collections {
            let mut items: Vec<Item> = Vec::new();
            // right now we only load <source> but we should in the future
            // potential also load <query>. Right now the test suite doesn't
            // exercise this.
            // https://github.com/w3c/qt3tests/issues/66
            for source in &collection.sources {
                let node = source.node(&self.base_dir, documents, &source.uri, base_uri)?;
                items.push(node.into());
            }
            collections.insert(collection.uri.clone(), items.into());
        }
        Ok(collections)
    }

    pub(crate) fn context_item(
        &self,
        documents: &mut Documents,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<Option<Item>> {
        for source in &self.sources {
            if let SourceRole::Context = source.role {
                let node = source.node(&self.base_dir, documents, &source.uri, base_uri)?;
                return Ok(Some(Item::from(node)));
            }
        }
        Ok(None)
    }

    pub(crate) fn variables(
        &self,
        documents: &mut Documents,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<context::Variables> {
        let mut variables = context::Variables::new();
        for source in &self.sources {
            if let SourceRole::Var(name) = &source.role {
                let name = &name[1..]; // without $
                let node = source.node(&self.base_dir, documents, &source.uri, base_uri)?;
                variables.insert(Name::name(name), Item::from(node).into());
            }
        }
        let mut documents = Documents::new();
        for param in &self.params {
            let select = (param.select.as_ref()).expect("param: missing select not supported");
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
            let result = query.execute_with_context(&mut documents, &dynamic_context)?;
            variables.insert(param.name.clone(), result);
        }
        Ok(variables)
    }
}

impl ContextLoadable<LoadContext> for EnvironmentSpec {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        let sources_query = Sources::load_with_context(queries, context)?;

        let name_query = queries.one("@name/string()", convert_string)?;
        let select_query = queries.option("@select/string()", convert_string)?;
        let as_query = queries.option("@as/string()", convert_string)?;
        let source_query = queries.option("@source/string()", convert_string)?;
        let declared_query = queries.option("@declared/string()", convert_string)?;

        let params_query = queries.many("param", move |documents, item| {
            let name = name_query.execute(documents, item)?;
            let select = select_query.execute(documents, item)?;
            let as_ = as_query.execute(documents, item)?;
            let source = source_query.execute(documents, item)?;
            let declared = declared_query.execute(documents, item)?;

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

        let uri_query = queries.one("@uri/string()", convert_string)?;
        let collection_sources_query = Sources::load_with_context(queries, context)?;
        let collections_query = queries.many("collection", move |documents, item| {
            let uri = uri_query.execute(documents, item)?;
            let sources = collection_sources_query.execute(documents, item)?;

            let collection = Collection {
                uri,
                sources: sources.sources,
                queries: Vec::new(),
                resources: Vec::new(),
            };

            Ok(collection)
        })?;

        // the environment base_dir is the same as the catalog/test set path,
        // but without the file name
        let path = context.path.parent().unwrap();
        let static_base_uri_query =
            queries.option("static-base-uri/@uri/string()", convert_string)?;
        let environment_query = queries.one(".", move |documents, item| {
            let sources = sources_query.execute(documents, item)?;
            let params = params_query.execute(documents, item)?;
            let static_base_uri = static_base_uri_query.execute(documents, item)?;
            let collections = collections_query.execute(documents, item)?;
            let environment_spec = EnvironmentSpec {
                base_dir: path.to_path_buf(),
                sources: sources.sources,
                params,
                static_base_uri,
                collections,
                ..Default::default()
            };

            Ok(environment_spec)
        })?;

        Ok(environment_query)
    }
}

#[cfg(test)]
mod tests {
    use crate::{environment::source::SourceContent, metadata::Metadata, ns::XPATH_TEST_NS};

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

        let context = LoadContext {
            path: PathBuf::from("bar/foo"),
            catalog_ns: XPATH_TEST_NS,
        };
        let environment_spec = EnvironmentSpec::load_from_xml_with_context(&xml, &context).unwrap();
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
                        uri: "a.xml".try_into().unwrap(),
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
                        uri: "b.xml".try_into().unwrap(),
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
                <source role="." uri="example"><content>Foo</content></source>
            </environment>"#,
            XPATH_TEST_NS
        );

        let context = LoadContext {
            path: PathBuf::from("bar/foo"),
            catalog_ns: XPATH_TEST_NS,
        };
        let environment_spec = EnvironmentSpec::load_from_xml_with_context(&xml, &context).unwrap();
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
                    uri: "example".try_into().unwrap(),
                    validation: None,
                },],
                ..Default::default()
            }
        )
    }
}
