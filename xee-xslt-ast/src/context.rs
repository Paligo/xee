use std::str::FromStr;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use rust_decimal::Decimal;
use xee_xpath_ast::{ast as xpath_ast, VariableNames, XPathParserContext};
use xee_xpath_ast::{Namespaces, FN_NAMESPACE};

use crate::{ast_core as ast, state::State};

/// Parser context is passed around. You can create new contexts as
/// for particular sub-trees.

#[derive(Debug, Clone)]
pub(crate) struct Context {
    prefixes: xot::Prefixes,
    // known variable names
    variable_names: HashSet<xpath_ast::Name>,

    default_collation: Vec<ast::Uri>,
    pub(crate) default_mode: ast::DefaultMode,
    default_validation: ast::DefaultValidation,
    pub(crate) expand_text: bool,
    version: Decimal,
    xpath_default_namespace: ast::Uri,
    // cumulative; the namespaces designated by exclude-result-prefixes and
    // extension-element-prefixes, resolved against the in-scope bindings of
    // the element bearing the attribute
    excluded_namespaces: HashSet<xot::NamespaceId>,
    extension_element_prefixes: Vec<ast::Prefix>,
    // the in-scope prefixes of the nearest enclosing literal result element
    literal_result_element_prefixes: xot::Prefixes,
}

impl Context {
    pub(crate) fn new(prefixes: xot::Prefixes) -> Self {
        let mut r = Self::empty();
        r.prefixes = prefixes;
        r
    }

    pub(crate) fn empty() -> Self {
        Self {
            prefixes: xot::Prefixes::new(),
            variable_names: HashSet::new(),
            default_collation: vec![
                "http://www.w3.org/2005/xpath-functions/collation/codepoint".to_string()
            ],
            default_mode: ast::DefaultMode::Unnamed,
            default_validation: ast::DefaultValidation::Strip,
            expand_text: false,
            version: Decimal::from_str("3.0").unwrap(),
            xpath_default_namespace: "".to_string(),
            excluded_namespaces: HashSet::new(),
            extension_element_prefixes: vec![],
            literal_result_element_prefixes: xot::Prefixes::new(),
        }
    }

    pub(crate) fn with_prefixes(&self, prefixes: &xot::Prefixes) -> Self {
        let mut expanded_prefixes = self.prefixes.clone();
        expanded_prefixes.extend(prefixes);
        Self {
            prefixes: expanded_prefixes,
            ..self.clone()
        }
    }

    pub(crate) fn with_variable_name(&self, name: &xpath_ast::Name) -> Self {
        let mut variable_names = self.variable_names.clone();
        variable_names.insert(name.clone());
        Self {
            variable_names,
            ..self.clone()
        }
    }

    pub(crate) fn with_static_standard(
        &self,
        namespaces: xot::Namespaces,
        static_standard: ast::StaticStandard,
    ) -> Self {
        let mut expanded_prefixes = self.prefixes.clone();
        expanded_prefixes.extend(
            namespaces
                .iter()
                .map(|(k, v)| (k, *v))
                .collect::<xot::Prefixes>(),
        );
        let xpath_default_namespace =
            if let Some(xpath_default_namespace) = static_standard.xpath_default_namespace {
                xpath_default_namespace
            } else {
                self.xpath_default_namespace.clone()
            };
        Self {
            prefixes: expanded_prefixes,
            xpath_default_namespace,
            ..self.clone()
        }
    }

    pub(crate) fn with_standard(
        &self,
        state: &State,
        namespaces: xot::Namespaces,
        standard: ast::Standard,
    ) -> Self {
        let mut expanded_prefixes = self.prefixes.clone();
        expanded_prefixes.extend(
            namespaces
                .iter()
                .map(|(k, v)| (k, *v))
                .collect::<xot::Prefixes>(),
        );
        let default_collation = if let Some(default_collation) = standard.default_collation {
            default_collation
        } else {
            self.default_collation.clone()
        };
        let default_mode = if let Some(default_mode) = standard.default_mode {
            default_mode
        } else {
            self.default_mode.clone()
        };
        let default_validation = if let Some(default_validation) = standard.default_validation {
            default_validation
        } else {
            self.default_validation.clone()
        };
        let expand_text = if let Some(expand_text) = standard.expand_text {
            expand_text
        } else {
            self.expand_text
        };
        let version = if let Some(version) = standard.version {
            version
        } else {
            self.version
        };
        let xpath_default_namespace =
            if let Some(xpath_default_namespace) = standard.xpath_default_namespace {
                xpath_default_namespace
            } else {
                self.xpath_default_namespace.clone()
            };
        // exclude-result-prefixes and extension-element-prefixes designate
        // namespaces by resolving each prefix against the bindings in scope
        // at the element bearing the attribute
        let mut excluded_namespaces = self.excluded_namespaces.clone();
        let exclude_prefix = |wanted: &str, excluded: &mut HashSet<xot::NamespaceId>| {
            // TODO: a prefix without a binding is a static error (XTSE0808)
            for (prefix_id, namespace_id) in &expanded_prefixes {
                if state.xot.prefix_str(*prefix_id) == wanted {
                    excluded.insert(*namespace_id);
                }
            }
        };
        if let Some(exclude_result_prefixes) = standard.exclude_result_prefixes {
            match exclude_result_prefixes {
                ast::ExcludeResultPrefixes::All => {
                    excluded_namespaces.extend(expanded_prefixes.values().copied());
                }
                ast::ExcludeResultPrefixes::Prefixes(prefixes) => {
                    for prefix in prefixes {
                        match prefix {
                            ast::ExcludeResultPrefix::Default => {
                                exclude_prefix("", &mut excluded_namespaces)
                            }
                            ast::ExcludeResultPrefix::Prefix(prefix) => {
                                exclude_prefix(&prefix, &mut excluded_namespaces)
                            }
                        }
                    }
                }
            }
        }
        if let Some(extension_element_prefixes) = &standard.extension_element_prefixes {
            for prefix in extension_element_prefixes {
                exclude_prefix(prefix, &mut excluded_namespaces);
            }
        }
        let extension_element_prefixes =
            if let Some(extension_element_prefixes) = standard.extension_element_prefixes {
                // TODO for now just add all prefixes. This isn't right.
                self.extension_element_prefixes
                    .iter()
                    .chain(extension_element_prefixes.iter())
                    .cloned()
                    .collect()
            } else {
                self.extension_element_prefixes.clone()
            };

        Self {
            prefixes: expanded_prefixes,
            default_collation,
            default_mode,
            default_validation,
            expand_text,
            version,
            xpath_default_namespace,
            excluded_namespaces,
            extension_element_prefixes,
            ..self.clone()
        }
    }

    /// A context for the children of a literal result element, which
    /// remembers the in-scope prefixes the element declares in the result.
    pub(crate) fn with_literal_result_element(&self) -> Self {
        Self {
            literal_result_element_prefixes: self.prefixes.clone(),
            ..self.clone()
        }
    }

    /// A context for content that constructs a separate tree (such as
    /// xsl:variable): its elements cannot rely on an enclosing literal
    /// result element to declare their namespaces.
    pub(crate) fn without_literal_result_element(&self) -> Self {
        Self {
            literal_result_element_prefixes: xot::Prefixes::new(),
            ..self.clone()
        }
    }

    /// The namespaces a literal result element declares in the result: its
    /// in-scope namespaces, except the XSLT namespace, excluded namespaces
    /// (exclude-result-prefixes and extension element prefixes), and
    /// namespaces already declared in the result by the nearest enclosing
    /// literal result element. See XSLT 3.0 section 11.1.3.
    pub(crate) fn literal_result_element_namespaces(&self, state: &State) -> Vec<(String, String)> {
        let mut namespaces = Vec::new();
        for (prefix_id, namespace_id) in &self.prefixes {
            if self.literal_result_element_prefixes.get(prefix_id) == Some(namespace_id) {
                continue;
            }
            if *namespace_id == state.names.xsl_ns
                || self.excluded_namespaces.contains(namespace_id)
            {
                continue;
            }
            let namespace = state.xot.namespace_str(*namespace_id);
            // an xmlns="" undeclaration is not a namespace
            if namespace.is_empty() {
                continue;
            }
            namespaces.push((
                state.xot.prefix_str(*prefix_id).to_string(),
                namespace.to_string(),
            ));
        }
        // sort for a stable declaration order
        namespaces.sort();
        namespaces
    }

    pub(crate) fn namespaces<'a>(&'a self, state: &'a State) -> Namespaces {
        let mut namespaces = HashMap::new();
        for (prefix, ns) in &self.prefixes {
            let prefix = state.xot.prefix_str(*prefix);
            let uri = state.xot.namespace_str(*ns);
            namespaces.insert(prefix.to_string(), uri.to_string());
        }
        Namespaces::new(
            namespaces,
            self.xpath_default_namespace.to_string(),
            FN_NAMESPACE.to_string(),
        )
    }

    pub(crate) fn variable_names(&self) -> &VariableNames {
        &self.variable_names
    }

    pub(crate) fn parser_context(&self, state: &State) -> XPathParserContext {
        let namespaces = self.namespaces(state);
        XPathParserContext::new(namespaces, self.variable_names.clone())
    }
}
