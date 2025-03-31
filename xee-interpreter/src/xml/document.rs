use std::sync::atomic;

use ahash::{HashMap, HashMapExt};
use iri_string::types::{IriStr, IriString};
use xot::Xot;

use super::Annotations;

static DOCUMENTS_COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

fn get_documents_id() -> usize {
    DOCUMENTS_COUNTER.fetch_add(1, atomic::Ordering::Relaxed)
}

/// Something went wrong loading [`Documents`]
#[derive(Debug)]
pub enum DocumentsError {
    /// An attempt as made to add a document with a URI that was already known.
    DuplicateUri(String),
    /// An error occurred loading the document XML (using the [`xot`] crate).
    Parse(xot::ParseError),
}

impl std::error::Error for DocumentsError {}

impl std::fmt::Display for DocumentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateUri(uri) => write!(f, "Duplicate URI: {}", uri),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl From<xot::ParseError> for DocumentsError {
    fn from(e: xot::ParseError) -> Self {
        Self::Parse(e)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub(crate) uri: Option<IriString>,
    root: xot::Node,
}

impl Document {
    /// The document root node
    pub fn root(&self) -> xot::Node {
        self.root
    }

    pub(crate) fn cleanup(&self, xot: &mut Xot) {
        xot.remove(self.root).unwrap();
    }
}

/// A collection of XML documents as can be used by XPath and XSLT.
///
/// This collection can be prepared before any XPath or XSLT processing begins.
///
/// Alternatively this collection can be added to incrementally during
/// processing using the `fn:doc` function for instance. Once a document under
/// a URL is present, it cannot be changed anymore.
///
/// The `fn:parse-xml` and `fn:parse-xml-fragment` functions can be used to
/// create new documents from strings without URLs.
#[derive(Debug, Clone)]
pub struct Documents {
    id: usize,
    annotations: Annotations,
    documents: Vec<Document>,
    by_uri: HashMap<IriString, DocumentHandle>,
    uri_by_document_node: HashMap<xot::Node, IriString>,
}

/// A handle to a document.
///
/// This is an identifier into a [`Documents`] collection. You can
/// freely copy it.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct DocumentHandle {
    pub(crate) documents_id: usize,
    pub(crate) id: usize,
}

impl Documents {
    /// Create a new empty collection of documents.
    pub fn new() -> Self {
        Self {
            id: get_documents_id(),
            annotations: Annotations::new(),
            documents: Vec::new(),
            by_uri: HashMap::new(),
            uri_by_document_node: HashMap::new(),
        }
    }

    /// Clean up all documents.
    pub fn cleanup(&mut self, xot: &mut Xot) {
        for document in &self.documents {
            document.cleanup(xot);
        }
        self.annotations.clear();
        self.documents.clear();
        self.by_uri.clear();
    }

    /// Add a string as an XML document. It can be designated with a URI.
    pub fn add_string(
        &mut self,
        xot: &mut Xot,
        uri: Option<&IriStr>,
        xml: &str,
    ) -> Result<DocumentHandle, DocumentsError> {
        let root = xot.parse(xml)?;
        self.add_root(xot, uri, root)
    }

    /// Add a string as an XML fragment.
    pub fn add_fragment_string(
        &mut self,
        xot: &mut Xot,
        xml: &str,
    ) -> Result<DocumentHandle, DocumentsError> {
        let root = xot.parse_fragment(xml)?;
        self.add_root(xot, None, root)
    }

    /// Add a root node of an XML document. Designate it with a URI.
    pub fn add_root(
        &mut self,
        xot: &Xot,
        uri: Option<&IriStr>,
        root: xot::Node,
    ) -> Result<DocumentHandle, DocumentsError> {
        if let Some(uri) = uri {
            if self.by_uri.contains_key(uri) {
                // duplicate URI is an error
                return Err(DocumentsError::DuplicateUri(uri.as_str().to_string()));
            }
        }

        let id = self.documents.len();
        let handle = DocumentHandle {
            documents_id: self.id,
            id,
        };
        self.documents.push(Document {
            uri: uri.map(|uri| uri.to_owned()),
            root,
        });
        if let Some(uri) = uri {
            self.by_uri.insert(uri.to_owned(), handle);
            self.uri_by_document_node.insert(root, uri.to_owned());
        }
        self.annotations.add(xot, root);

        Ok(handle)
    }

    /// Obtain a document by handle
    pub fn get_by_handle(&self, handle: DocumentHandle) -> Option<&Document> {
        // only works if the handle is from this collection
        if handle.documents_id != self.id {
            return None;
        }
        self.documents.get(handle.id)
    }

    /// Obtain document node by handle
    pub fn get_node_by_handle(&self, handle: DocumentHandle) -> Option<xot::Node> {
        Some(self.get_by_handle(handle)?.root)
    }

    /// Obtain a document by URI
    ///
    /// It's only possible to obtain a document by URI if it was added with a URI.
    pub fn get_by_uri(&self, uri: &IriStr) -> Option<&Document> {
        let handle = self.by_uri.get(uri)?;
        self.get_by_handle(*handle)
    }

    /// Obtain document node by URI
    pub fn get_node_by_uri(&self, uri: &IriStr) -> Option<xot::Node> {
        Some(self.get_by_uri(uri)?.root)
    }

    /// Obtain document URI by document node.
    ///
    /// This only returns a URI if the document was added with a URI.
    pub fn get_uri_by_document_node(&self, node: xot::Node) -> Option<IriString> {
        self.uri_by_document_node.get(&node).cloned()
    }

    /// How many documents are stored.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Is the collection empty?
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get the annotations object
    pub(crate) fn annotations(&self) -> &Annotations {
        &self.annotations
    }
}

impl Default for Documents {
    fn default() -> Self {
        Self::new()
    }
}
