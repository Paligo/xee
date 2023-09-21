use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use icu_provider_blob::BlobDataProvider;
use xee_xpath_ast::ast;
use xee_xpath_ast::{Namespaces, Parsers};

use crate::error;
use crate::function::StaticFunctions;
use crate::string::provider;
use crate::string::{Collation, Collations};

#[derive(Debug)]
pub struct StaticContext<'a> {
    pub(crate) namespaces: &'a Namespaces<'a>,
    // XXX need to add in type later
    pub(crate) variables: Vec<ast::Name>,
    pub(crate) functions: StaticFunctions,
    provider: BlobDataProvider,
    pub(crate) collations: RefCell<Collations>,
    pub(crate) parsers: Parsers,
}

impl<'a> StaticContext<'a> {
    pub fn new(namespaces: &'a Namespaces<'a>) -> Self {
        Self {
            namespaces,
            variables: Vec::new(),
            functions: StaticFunctions::new(),
            collations: RefCell::new(Collations::new()),
            provider: provider(),
            parsers: Parsers::new(),
        }
    }

    pub fn with_variable_names(namespaces: &'a Namespaces<'a>, variables: &[ast::Name]) -> Self {
        Self {
            namespaces,
            variables: variables.to_vec(),
            functions: StaticFunctions::new(),
            collations: RefCell::new(Collations::new()),
            provider: provider(),
            parsers: Parsers::new(),
        }
    }

    pub(crate) fn default_collation(&self) -> error::Result<Rc<Collation>> {
        self.collation(self.default_collation_uri())
    }

    pub(crate) fn default_collation_uri(&self) -> &str {
        "http://www.w3.org/2005/xpath-functions/collation/codepoint"
    }

    pub(crate) fn collation(&self, uri: &str) -> error::Result<Rc<Collation>> {
        // TODO: supply static base URI
        self.collations
            .borrow_mut()
            .load(self.provider.clone(), None, uri)
    }

    pub(crate) fn icu_provider(&self) -> &BlobDataProvider {
        &self.provider
    }
}
