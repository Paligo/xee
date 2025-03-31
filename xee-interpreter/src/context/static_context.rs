use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::LazyLock;

use iri_string::types::IriAbsoluteStr;
use iri_string::types::IriAbsoluteString;
use iri_string::types::IriReferenceStr;
use xee_name::{Namespaces, VariableNames};
use xee_xpath_ast::ast;
use xee_xpath_ast::XPathParserContext;

use crate::error;
use crate::function;
use crate::string::{Collation, Collations};

static STATIC_FUNCTIONS: LazyLock<function::StaticFunctions> =
    LazyLock::new(function::StaticFunctions::new);

// use lazy static to initialize the default collation
static DEFAULT_COLLATION: LazyLock<IriAbsoluteString> = LazyLock::new(|| {
    "http://www.w3.org/2005/xpath-functions/collation/codepoint"
        .try_into()
        .unwrap()
});

#[derive(Debug)]
pub struct StaticContext {
    parser_context: XPathParserContext,
    functions: &'static function::StaticFunctions,
    // TODO: try to make collations static
    collations: RefCell<Collations>,
    static_base_uri: Option<IriAbsoluteString>,
}

impl Default for StaticContext {
    fn default() -> Self {
        Self::new(Namespaces::default(), VariableNames::default(), None)
    }
}

impl From<XPathParserContext> for StaticContext {
    fn from(parser_context: XPathParserContext) -> Self {
        Self {
            parser_context,
            functions: &STATIC_FUNCTIONS,
            collations: RefCell::new(Collations::new()),
            static_base_uri: None,
        }
    }
}

impl StaticContext {
    pub(crate) fn new(
        namespaces: Namespaces,
        variable_names: VariableNames,
        static_base_uri: Option<IriAbsoluteString>,
    ) -> Self {
        Self {
            parser_context: XPathParserContext::new(namespaces, variable_names),
            functions: &STATIC_FUNCTIONS,
            collations: RefCell::new(Collations::new()),
            static_base_uri,
        }
    }

    pub fn from_namespaces(namespaces: Namespaces) -> Self {
        Self::new(namespaces, VariableNames::default(), None)
    }

    pub fn namespaces(&self) -> &Namespaces {
        &self.parser_context.namespaces
    }

    pub fn variable_names(&self) -> &VariableNames {
        &self.parser_context.variable_names
    }

    pub fn default_collation(&self) -> error::Result<Rc<Collation>> {
        self.collation(self.default_collation_uri())
    }

    pub fn default_collation_uri(&self) -> &IriReferenceStr {
        DEFAULT_COLLATION.as_ref()
    }

    pub(crate) fn resolve_collation_str(
        &self,
        collation: Option<&str>,
    ) -> error::Result<Rc<Collation>> {
        let collation: Option<&IriReferenceStr> = if let Some(collation) = collation {
            collation.try_into().ok()
        } else {
            None
        };
        self.collation(collation.unwrap_or(self.default_collation_uri()))
    }

    pub fn static_base_uri(&self) -> Option<&IriAbsoluteStr> {
        self.static_base_uri.as_deref()
    }

    pub(crate) fn collation(&self, uri: &IriReferenceStr) -> error::Result<Rc<Collation>> {
        self.collations
            .borrow_mut()
            .load(self.static_base_uri(), uri)
    }

    /// Given an XPath string, parse into an XPath AST
    ///
    /// This uses the namespaces and variable names with which
    /// this static context has been initialized.
    pub fn parse_xpath(&self, s: &str) -> Result<ast::XPath, xee_xpath_ast::ParserError> {
        self.parser_context.parse_xpath(s)
    }

    /// Parse an XPath string as it would appear in an XSLT value template.
    /// This means it should have a closing `}` following the xpath expression.
    pub fn parse_value_template_xpath(
        &self,
        s: &str,
    ) -> Result<ast::XPath, xee_xpath_ast::ParserError> {
        self.parser_context.parse_value_template_xpath(s)
    }

    /// Get a static function by id
    pub fn function_by_id(
        &self,
        static_function_id: function::StaticFunctionId,
    ) -> &function::StaticFunction {
        self.functions.get_by_index(static_function_id)
    }

    /// Get a static function by name and arity
    pub fn function_id_by_name(
        &self,
        name: &xot::xmlname::OwnedName,
        arity: u8,
    ) -> Option<function::StaticFunctionId> {
        self.functions.get_by_name(name, arity)
    }

    /// Get an internal static function by name and arity
    pub fn function_id_by_internal_name(
        &self,
        name: &xot::xmlname::OwnedName,
        arity: u8,
    ) -> Option<function::StaticFunctionId> {
        self.functions.get_by_internal_name(name, arity)
    }
}
