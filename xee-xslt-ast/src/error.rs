use crate::{ast_core::Span, name::XmlName, value_template};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AttributeError {
    // Expected attribute of name, not found (element span)
    NotFound { name: XmlName, span: Span },
    // Did not expect attribute of name (attribute span)
    Unexpected { name: XmlName, span: Span },
    // The value of an attribute was invalid
    Invalid { value: String, span: Span },
    // An eqname was invalid
    InvalidEqName { value: String, span: Span },
    // XPath parser error
    XPathParser(xee_xpath_ast::ParserError),
    // A value template could not be parsed
    ValueTemplate(value_template::Error),
    // Internal error; should not happen
    Internal,
}

impl From<xee_xpath_ast::ParserError> for AttributeError {
    fn from(e: xee_xpath_ast::ParserError) -> Self {
        Self::XPathParser(e)
    }
}

impl From<value_template::Error> for AttributeError {
    fn from(e: value_template::Error) -> Self {
        Self::ValueTemplate(e)
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ElementError {
    // Did not expect this node
    Unexpected { span: Span },
    // Did not expect end TODO: how to get span info?
    UnexpectedEnd,
    // An attribute of the element was invalid
    Attribute(AttributeError),
    ValueTemplate(value_template::Error),
    // XPath runtime error
    XPathRunTime(xee_xpath_compiler::error::SpannedError),
    // internal error, should not happen
    Internal,
}

impl From<AttributeError> for ElementError {
    fn from(error: AttributeError) -> Self {
        Self::Attribute(error)
    }
}

impl From<xee_xpath_compiler::error::SpannedError> for ElementError {
    fn from(e: xee_xpath_compiler::error::SpannedError) -> Self {
        Self::XPathRunTime(e)
    }
}

impl From<value_template::Error> for ElementError {
    fn from(e: value_template::Error) -> Self {
        Self::ValueTemplate(e)
    }
}

impl From<xot::Error> for ElementError {
    fn from(_e: xot::Error) -> Self {
        Self::Internal
    }
}
