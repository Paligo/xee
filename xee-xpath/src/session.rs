use crate::{
    documents::{Documents, InnerDocuments},
    error::DocumentsError,
    queries::Queries,
    DocumentHandle,
};

/// A session in which queries can be executed
///
/// You construct one using the [`Queries::session`] method.
#[derive(Debug)]
pub struct Session<'namespaces> {
    pub(crate) queries: &'namespaces Queries<'namespaces>,
    pub(crate) dynamic_context: xee_interpreter::context::DynamicContext<'namespaces>,
    pub(crate) documents: InnerDocuments,
}

impl<'namespaces> Session<'namespaces> {
    pub(crate) fn new(queries: &'namespaces Queries<'namespaces>, documents: Documents) -> Self {
        let dynamic_context = xee_interpreter::context::DynamicContext::from_owned_documents(
            &queries.static_context,
            documents.documents,
        );
        Self {
            queries,
            dynamic_context,
            documents: documents.inner,
        }
    }

    /// Add document to the session
    pub fn add_document_by_uri(&mut self, uri: &str, xml: &str) -> Result<(), DocumentsError> {
        // TODO: duplication with Documents. Should rewrite interpreter documents
        // to use the document handle API to resolve this
        let uri = xee_interpreter::xml::Uri::new(uri);
        if self.documents.document_uris.contains(&uri) {
            // duplicate URI is an error
            return Err(DocumentsError::DuplicateUri(uri.as_str().to_string()));
        }
        // let id = self.documents.document_uris.len();

        self.dynamic_context
            .documents
            .borrow_mut()
            .add(&mut self.documents.xot, &uri, xml)?;
        self.documents.document_uris.push(uri);
        Ok(())
    }

    /// Get the document root node by URI
    pub fn get_document_node_by_uri(&self, uri: &str) -> Option<xot::Node> {
        let uri = xee_interpreter::xml::Uri::new(uri);
        let borrowed_documents = self.dynamic_context.documents().borrow();
        let document = borrowed_documents.get(&uri)?;
        Some(document.root())
    }

    /// Amount of documents we have
    pub fn documents_count(&self) -> usize {
        self.documents.document_uris.len()
    }
}
