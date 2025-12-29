use std::str::FromStr;

use ahash::{HashSet, HashSetExt};
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
    // cumulative
    exclude_result_prefixes: ast::ExcludeResultPrefixes,
    extension_element_prefixes: Vec<ast::Prefix>,
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
            exclude_result_prefixes: ast::ExcludeResultPrefixes::Prefixes(vec![]),
            extension_element_prefixes: vec![],
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
        let exclude_result_prefixes =
            if let Some(exclude_result_prefixes) = standard.exclude_result_prefixes {
                self.exclude_result_prefixes
                    .combine(exclude_result_prefixes)
            } else {
                self.exclude_result_prefixes.clone()
            };
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
            exclude_result_prefixes,
            extension_element_prefixes,
            ..self.clone()
        }
    }

    pub(crate) fn namespaces<'a>(&'a self, state: &'a State) -> Namespaces {
        let mut namespaces = Namespaces::default_namespaces();
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
