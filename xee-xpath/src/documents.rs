use iri_string::types::IriStr;
use xee_interpreter::{
    context::DocumentsRef,
    context::TypeTableRef,
    xml::{DocumentHandle, DocumentsError},
};
use xot::Xot;

/// A collection of XML documents as can be used by XPath and XSLT.
///
/// This collection can be prepared before any XPath or XSLT processing begins.
///
/// Alternatively this collection can be added to incrementally during
/// processing using the `fn:doc` function for instance. Once a document under
/// a URL is present, it won't be changed.
#[derive(Debug)]
pub struct Documents {
    pub(crate) xot: Xot,
    pub(crate) documents: DocumentsRef,
    pub(crate) type_table: TypeTableRef,
}

impl Documents {
    /// Create a new empty collection of documents.
    pub fn new() -> Self {
        Self {
            xot: Xot::new(),
            documents: DocumentsRef::new(),
            type_table: TypeTableRef::new(),
        }
    }

    /// Load a string as an XML document. Designate it with a URI.
    ///
    /// Something may go wrong during processing of the XML document; this is
    /// a [`xot::Error`].
    pub fn add_string(
        &mut self,
        uri: &IriStr,
        xml: &str,
    ) -> Result<DocumentHandle, DocumentsError> {
        self.documents
            .borrow_mut()
            .add_string(&mut self.xot, Some(uri), xml)
    }

    /// Load a string as an XML document without designating it with a URI.
    ///
    /// Something may go wrong during processing of the XML document; this is
    /// a [`xot::Error`].
    pub fn add_string_without_uri(&mut self, xml: &str) -> Result<DocumentHandle, DocumentsError> {
        self.documents
            .borrow_mut()
            .add_string(&mut self.xot, None, xml)
    }

    /// Given a handle give back the document node
    pub fn document_node(&self, handle: DocumentHandle) -> Option<xot::Node> {
        self.documents.borrow().get_node_by_handle(handle)
    }

    /// Get a reference to the documents
    pub fn documents(&self) -> &DocumentsRef {
        &self.documents
    }

    /// Get a reference to the schema type table.
    pub fn type_table(&self) -> &TypeTableRef {
        &self.type_table
    }

    /// Get a reference to the Xot arena
    pub fn xot(&self) -> &Xot {
        &self.xot
    }

    /// Get a mutable reference to the Xot arena
    pub fn xot_mut(&mut self) -> &mut Xot {
        &mut self.xot
    }
}

impl Default for Documents {
    fn default() -> Self {
        Self::new()
    }
}
