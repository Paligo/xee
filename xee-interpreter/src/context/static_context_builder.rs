use ahash::{HashMap, HashSet};
use iri_string::types::IriAbsoluteString;
use xee_name::Namespaces;
use xot::xmlname::OwnedName;

use crate::context;

#[derive(Debug, Clone, Default)]
pub struct StaticContextBuilder<'a> {
    variable_names: Vec<OwnedName>,
    namespaces: HashMap<&'a str, &'a str>,
    disabled_functions: HashSet<OwnedName>,
    default_element_namespace: &'a str,
    default_function_namespace: &'a str,
    static_base_uri: Option<IriAbsoluteString>,
}

impl<'a> StaticContextBuilder<'a> {
    /// Set the variable names that the XPath expression expects.
    ///
    /// They should be supplied in the order that they would be passed into if
    /// the XPath expression was a function.
    ///
    /// Calling this multiple times will override the variable names.
    pub fn variable_names(
        &mut self,
        variable_names: impl IntoIterator<Item = OwnedName>,
    ) -> &mut Self {
        self.variable_names = variable_names.into_iter().collect();
        self
    }

    /// Set the namespace prefixes that the XPath expression can use.
    ///
    /// This is an iterable of tuples where the first element is the prefix and
    /// the second element is the namespace URI.
    ///
    /// If a prefix is empty, it sets the default namespace.
    ///
    /// Calling this multiple times will override the namespaces.
    pub fn namespaces(
        &mut self,
        namespaces: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> &mut Self {
        for (prefix, uri) in namespaces {
            self.add_namespace(prefix, uri);
        }
        self
    }

    /// Add a namespace prefix that the XPath expression can use.
    pub fn add_namespace(&mut self, prefix: &'a str, uri: &'a str) -> &mut Self {
        if prefix.is_empty() {
            self.default_element_namespace = uri;
        } else {
            self.namespaces.insert(prefix, uri);
        }
        self
    }

    /// Set the default namespace for element references in the XPath expression.
    pub fn default_element_namespace(&mut self, default_element_namespace: &'a str) -> &mut Self {
        self.default_element_namespace = default_element_namespace;
        self
    }

    /// Set the default namespace for function references in the XPath expression.
    pub fn default_function_namespace(&mut self, default_function_namespace: &'a str) -> &mut Self {
        self.default_function_namespace = default_function_namespace;
        self
    }

    /// Set the static base URI
    pub fn static_base_uri(&mut self, static_base_uri: Option<IriAbsoluteString>) -> &mut Self {
        self.static_base_uri = static_base_uri;
        self
    }

    /// Disable a function name in this static context.
    pub fn disable_function(&mut self, name: OwnedName) -> &mut Self {
        self.disabled_functions.insert(name);
        self
    }

    /// Build the static context.
    ///
    /// This will always include the default known namespaces for
    /// XPath, and the default function namespace will be the `fn` namespace
    /// if not set.
    pub fn build(&self) -> context::StaticContext {
        let mut namespaces = Namespaces::default_namespaces();
        for (prefix, uri) in &self.namespaces {
            namespaces.insert(prefix.to_string(), uri.to_string());
        }
        let default_function_namespace = if !self.default_function_namespace.is_empty() {
            self.default_function_namespace
        } else {
            Namespaces::FN_NAMESPACE
        };
        let namespaces = xee_name::Namespaces::new(
            namespaces,
            self.default_element_namespace.to_string(),
            default_function_namespace.to_string(),
        );
        let variable_names = self.variable_names.clone().into_iter().collect();
        context::StaticContext::new(
            namespaces,
            variable_names,
            self.disabled_functions.clone(),
            self.static_base_uri.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use ahash::HashSet;

    use super::*;

    #[test]
    fn test_variable_names() {
        let mut builder = StaticContextBuilder::default();
        let foo = OwnedName::new("foo".to_string(), "".to_string(), "".to_string());
        let bar = OwnedName::new("bar".to_string(), "".to_string(), "".to_string());
        builder.variable_names([foo.clone(), bar.clone()]);
        assert_eq!(builder.variable_names, vec![foo, bar]);
    }

    #[test]
    fn test_default_behavior() {
        let builder = StaticContextBuilder::default();
        let static_context = builder.build();
        assert_eq!(static_context.namespaces().default_element_namespace(), "");
        assert_eq!(
            static_context.namespaces().default_function_namespace,
            Namespaces::FN_NAMESPACE
        );
        assert_eq!(static_context.variable_names(), &HashSet::default());
        assert_eq!(
            static_context.namespaces().by_prefix("xml"),
            Some("http://www.w3.org/XML/1998/namespace")
        );
    }
}
