use iri_string::types::IriStr;
use xee_interpreter::{
    context::{DocumentsRef, BorrowWith},
    xml::{DocumentHandle, DocumentsError},
};
use xot::Xot;

/// A collection of XML documents and their nodes that can be used by XPath and
/// XSLT.
///
/// This collection can be prepared before any XPath or XSLT processing begins.
///
/// Alternatively this collection can be added to incrementally during
/// processing using the `fn:doc` function for instance. Once a document under
/// a URL is present, it won't be changed.
///
/// The nodes from the added documents are kept in a [`Xot`] arena and can be
/// accessed through the `fn:xot` and `fn:xot_mut` methods.
///
/// The `fn:documents` method returns a reference to the
/// [`xee_interpreter::xml::Documents`] collection, which can be
/// used to look up the added [`xee_interpreter::xml::Document`] items
/// by URI or handle.
#[derive(Debug)]
pub struct Documents {
    // The Xot arena holding all nodes of the documents in the collection.
    pub(crate) xot: Xot,
    // A reference to the underlaying collection of XML documents
    // so they can be looked up by URI or handle. Each Document stores the
    // URI and root node of the XML data.
    pub(crate) documents: DocumentsRef,
}

impl Documents {
    /// Create a new empty collection of documents.
    pub fn new() -> Self {
        Self {
            xot: Xot::new(),
            documents: DocumentsRef::new(),
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

    /// Get a reference to the documents ([`xee_interpreter::xml::Documents`])
    pub fn documents(&self) -> &DocumentsRef {
        &self.documents
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
