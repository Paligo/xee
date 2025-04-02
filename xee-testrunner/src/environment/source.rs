use anyhow::Result;
use iri_string::types::{IriAbsoluteStr, IriReferenceStr, IriReferenceString, IriString};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use xee_xpath::{context, Documents, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::catalog::LoadContext;
use crate::metadata::Metadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) metadata: Metadata,
    // note that in a collection source the role can be ommitted, so
    // we may need to define this differently
    pub(crate) role: SourceRole,
    pub(crate) content: SourceContent,
    pub(crate) uri: IriReferenceString,
    pub(crate) validation: Option<Validation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceContent {
    Path(PathBuf),
    String(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Validation {
    Strict,
    Lax,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceRole {
    Context,
    Var(String),
    Doc(IriReferenceString), // URI
}

impl Source {
    pub(crate) fn node(
        &self,
        base_dir: &Path,
        documents: &mut Documents,
        uri: &IriReferenceStr,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<xot::Node> {
        let uri: IriString = if let Some(base_uri) = base_uri {
            uri.resolve_against(base_uri).into()
        } else {
            panic!("Cannot resolve relative URL")
        };

        match &self.content {
            SourceContent::Path(path) => {
                // this path resolution code is decidedly ugly
                // TODO: would be nice if we could get rid of options somewhere
                // down the line earlier and resolve earlier.
                let full_path = base_dir.join(path);
                // try to get the cached version of the document
                {
                    // scope borrowed_documents so we drop it afterward
                    let borrowed_documents = documents.documents().borrow();

                    let root = borrowed_documents.get_node_by_uri(&uri);
                    if let Some(root) = root {
                        return Ok(root);
                    }
                }

                // could not get cached version, so load up document
                let xml_file = File::open(&full_path)?;
                let mut buf_reader = BufReader::new(xml_file);
                let mut xml = String::new();
                buf_reader.read_to_string(&mut xml)?;

                let documents_ref = documents.documents().clone();
                let handle =
                    documents_ref
                        .borrow_mut()
                        .add_string(documents.xot_mut(), Some(&uri), &xml)?;
                Ok(documents
                    .documents()
                    .borrow()
                    .get_node_by_handle(handle)
                    .unwrap())
            }
            SourceContent::String(value) => {
                // we don't try to get a cached version of the document, as
                // that would be different each time. we just add it to documents
                // and return it
                // TODO: is this right?
                let documents_ref = documents.documents().clone();
                let handle = documents_ref.borrow_mut().add_string(
                    documents.xot_mut(),
                    Some(&uri),
                    value,
                )?;
                Ok(documents
                    .documents()
                    .borrow()
                    .get_node_by_handle(handle)
                    .unwrap())
            }
        }
    }
}

pub(crate) struct Sources {
    pub(crate) sources: Vec<Source>,
}

impl ContextLoadable<LoadContext> for Sources {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        let file_query = queries.option("@file/string()", convert_string)?;
        let content_query = queries.one("content/string()", convert_string)?;
        let role_query = queries.option("@role/string()", convert_string)?;
        let uri_query = queries.option("@uri/string()", convert_string)?;
        let metadata_query = Metadata::load_with_context(queries, context)?;

        let sources_query = queries.many("source", move |documents, item| {
            let content = if let Some(file) = file_query.execute(documents, item)? {
                SourceContent::Path(PathBuf::from(file))
            } else {
                // look for content inside
                let s = content_query.execute(documents, item)?;
                SourceContent::String(s)
            };
            let role = role_query.execute(documents, item)?;
            let uri = uri_query.execute(documents, item)?;

            let uri: IriReferenceString = if let Some(uri) = uri {
                uri.try_into().unwrap()
            } else {
                match &content {
                    // if there is no uri attribute, use the file attribute as the url
                    SourceContent::Path(path) => {
                        let uri = path.to_string_lossy().to_string();
                        uri.try_into().unwrap()
                    }
                    SourceContent::String(_) => {
                        panic!("Cannot have a source without a URI or file attribute")
                    }
                }
            };

            let metadata = metadata_query.execute(documents, item)?;

            let source = if let Some(role) = role {
                if role == "." {
                    Source {
                        metadata: metadata.clone(),
                        role: SourceRole::Context,
                        content: content.clone(),
                        uri,
                        validation: None,
                    }
                } else {
                    Source {
                        metadata: metadata.clone(),
                        role: SourceRole::Var(role),
                        content: content.clone(),
                        uri,
                        validation: None,
                    }
                }
            } else {
                Source {
                    metadata,
                    role: SourceRole::Doc(uri.clone()),
                    content,
                    uri,
                    // TODO
                    validation: None,
                }
            };

            Ok(source)
        })?;

        let all_sources_query = queries.one(".", move |documents, item| {
            let sources = sources_query.execute(documents, item)?;
            Ok(Sources { sources })
        })?;
        Ok(all_sources_query)
    }
}
