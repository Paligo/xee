use ahash::{HashMap, HashMapExt};
use std::sync::LazyLock;

/// The XPath FN namespace URI
pub const FN_NAMESPACE: &str = "http://www.w3.org/2005/xpath-functions";
/// The XML Schema XS namespace URI
pub const XS_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

const STATIC_NAMESPACES: [(&str, &str); 7] = [
    ("xs", XS_NAMESPACE),
    ("fn", FN_NAMESPACE),
    ("math", "http://www.w3.org/2005/xpath-functions/math"),
    ("map", "http://www.w3.org/2005/xpath-functions/map"),
    ("array", "http://www.w3.org/2005/xpath-functions/array"),
    ("err", "http://www.w3.org/2005/xqt-errors"),
    ("output", "http://www.w3.org/2010/xslt-xquery-serialization"),
];

/// Static default namespaces.
pub static DEFAULT_NAMESPACES: LazyLock<Namespaces> = LazyLock::new(|| Default::default());

/// Declared namespaces.
#[derive(Debug, Clone)]
pub struct Namespaces {
    namespaces: HashMap<String, String>,
    /// The default namespace for elements in XPath expressions.
    pub default_element_namespace: String,
    /// The default namespace for functions in XPath expressions.
    pub default_function_namespace: String,
}

impl Namespaces {
    /// The XPath FN namespace URI
    pub const FN_NAMESPACE: &'static str = FN_NAMESPACE;

    /// Create a new namespace struct.
    pub fn new(
        namespaces: HashMap<String, String>,
        default_element_namespace: String,
        default_function_namespace: String,
    ) -> Self {
        Self {
            namespaces,
            default_element_namespace,
            default_function_namespace,
        }
    }

    /// The default known namespaces for XPath.
    pub fn default_namespaces() -> HashMap<String, String> {
        let mut namespaces = HashMap::new();
        namespaces.insert("xml".to_string(), XML_NAMESPACE.to_string());
        for (prefix, uri) in STATIC_NAMESPACES.into_iter() {
            namespaces.insert(prefix.to_string(), uri.to_string());
        }
        namespaces
    }

    /// Add a list of namespace declarations (prefix, uri) to the namespace
    /// store.
    pub fn add(&mut self, namespace_pairs: &[(&str, &str)]) {
        for (prefix, namespace) in namespace_pairs {
            if prefix.is_empty() {
                self.default_element_namespace = namespace.to_string();
            } else {
                self.namespaces
                    .insert(prefix.to_string(), namespace.to_string());
            }
        }
    }

    /// Get the namespace URI for a given prefix.
    #[inline]
    pub fn by_prefix(&self, prefix: &str) -> Option<&str> {
        self.namespaces.get(prefix).map(String::as_str)
    }

    /// Get the default element namespace.
    #[inline]
    pub fn default_element_namespace(&self) -> &str {
        self.default_element_namespace.as_str()
    }
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new(
            Self::default_namespaces(),
            "".to_string(),
            FN_NAMESPACE.to_string(),
        )
    }
}

/// A trait for looking up namespace URIs by prefix.
pub trait NamespaceLookup {
    /// Get the namespace URI for a given prefix.
    fn by_prefix(&self, prefix: &str) -> Option<&str>;
}

impl NamespaceLookup for Namespaces {
    fn by_prefix(&self, prefix: &str) -> Option<&str> {
        self.namespaces.get(prefix).map(String::as_str)
    }
}

impl<T: NamespaceLookup> NamespaceLookup for &T {
    fn by_prefix(&self, prefix: &str) -> Option<&str> {
        (**self).by_prefix(prefix)
    }
}
