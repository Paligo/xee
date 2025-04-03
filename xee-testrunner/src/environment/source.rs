use anyhow::Result;
use iri_string::types::{IriAbsoluteStr, IriReferenceStr, IriReferenceString, IriString};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use xee_xpath::{context, Documents, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::catalog::LoadContext;
use crate::metadata::Metadata;
use crate::paths::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) metadata: Metadata,
    // note that in a collection source the role can be ommitted, so
    // we may need to define this differently
    pub(crate) role: SourceRole,
    pub(crate) content: SourceContent,
    // this can be optional at least in XSLT mode
    pub(crate) validation: Option<Validation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceContent {
    // load from file
    Path(PathBuf),
    // load from directly included content (XSLT only)
    Content(String),
    // execute string as xpath expression, should result in singleton (XSLT only)
    Select(String),
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
    Var(String),                       // only in XPath
    Doc(IriReferenceString),           // URI
    ContextAndDoc(IriReferenceString), // context & doc combined
}

impl Source {
    pub(crate) fn node(
        &self,
        base_dir: &Path,
        documents: &mut Documents,
        base_uri: Option<&IriAbsoluteStr>,
    ) -> Result<xot::Node> {
        // if we have a role that requires a URI we need to resolve it
        let uri: Option<IriString> = match &self.role {
            SourceRole::Doc(uri) | SourceRole::ContextAndDoc(uri) => {
                if let Some(base_uri) = base_uri {
                    Some(uri.resolve_against(base_uri).into())
                } else {
                    panic!("Cannot resolve relative URL")
                }
            }
            _ => None,
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

                    // when we load something from a path, we first check if we
                    // happen to know it under a URI already
                    if let Some(uri) = &uri {
                        // if we know it, we try to look it up
                        let root = borrowed_documents.get_node_by_uri(uri);
                        if let Some(root) = root {
                            return Ok(root);
                        }
                    }
                }

                // could not get cached version, so load up document
                let xml_file = File::open(&full_path)?;
                let mut buf_reader = BufReader::new(xml_file);
                let mut xml = String::new();
                buf_reader.read_to_string(&mut xml)?;

                let documents_ref = documents.documents().clone();
                let handle = documents_ref.borrow_mut().add_string(
                    documents.xot_mut(),
                    uri.as_deref(),
                    &xml,
                )?;
                Ok(documents
                    .documents()
                    .borrow()
                    .get_node_by_handle(handle)
                    .unwrap())
            }
            SourceContent::Content(value) => {
                // we don't try to get a cached version of the document, as
                // that would be different each time. we just add it to documents
                // and return it
                // TODO: is this right?
                let documents_ref = documents.documents().clone();
                let handle = documents_ref.borrow_mut().add_string(
                    documents.xot_mut(),
                    uri.as_deref(),
                    value,
                )?;
                Ok(documents
                    .documents()
                    .borrow()
                    .get_node_by_handle(handle)
                    .unwrap())
            }
            SourceContent::Select(_value) => {
                todo!("Don't know yet how to execute xpath here")
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

        let xslt_select_query = queries.option("@select/string()", convert_string)?;
        let xslt_content_query = queries.option("content/string()", convert_string)?;
        let sources_query = queries.many("source", move |documents, item| {
            let content = if let Some(file) = file_query.execute(documents, item)? {
                SourceContent::Path(PathBuf::from(file))
            } else {
                // HACK: we'd prefer to avoid mode dependence in the
                // code, but unfortunately source is parsed differently
                // based on the mode and this is the easiest way
                match context.mode {
                    Mode::XPath => {
                        // if we're in xpath mode, we take the content inside as an xpath expression
                        let s = content_query.execute(documents, item)?;
                        SourceContent::Content(s)
                    }
                    Mode::Xslt => {
                        panic!("no xslt yet");
                    }
                }
            };
            let role = role_query.execute(documents, item)?;
            let uri = uri_query.execute(documents, item)?;

            let uri: Option<IriReferenceString> = if let Some(uri) = uri {
                Some(uri.try_into().unwrap())
            } else {
                // HACK: this is a weird hack. if there's no uri attribute
                // then we wildly turn the path into the URI.
                // This is required for a few tests that depend on
                // the environment works-mod for instance,
                // which even it doesn't have a URI attribute in its
                // source, still seems fn-document-uri-20 to result in
                // a document with works-mod in its URI
                match &content {
                    // if there is no uri attribute, use the file attribute as the url
                    SourceContent::Path(path) => {
                        let uri = path.to_string_lossy().to_string();
                        Some(uri.try_into().unwrap())
                    }
                    SourceContent::Content(_) | SourceContent::Select(_) => None,
                }
            };

            let metadata = metadata_query.execute(documents, item)?;

            let source = if let Some(role) = role {
                if role == "." {
                    // it's possible to have a uri and a . role
                    // at the same time
                    if let Some(uri) = uri {
                        Source {
                            metadata: metadata.clone(),
                            role: SourceRole::ContextAndDoc(uri),
                            content: content.clone(),
                            validation: None,
                        }
                    } else {
                        Source {
                            metadata: metadata.clone(),
                            role: SourceRole::Context,
                            content: content.clone(),
                            validation: None,
                        }
                    }
                } else {
                    Source {
                        metadata: metadata.clone(),
                        role: SourceRole::Var(role),
                        content: content.clone(),
                        validation: None,
                    }
                }
            } else {
                if let Some(uri) = uri {
                    Source {
                        metadata: metadata.clone(),
                        role: SourceRole::ContextAndDoc(uri),
                        content: content.clone(),
                        validation: None,
                    }
                } else {
                    Source {
                        metadata: metadata.clone(),
                        role: SourceRole::Context,
                        content: content.clone(),
                        validation: None,
                    }
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
