use ibig::error::OutOfBoundsError;
use xee_xpath_ast::ParserError;

use crate::span::SourceSpan;

use super::Error;

/// An error code with an optional source span.
///
/// Also known as `SpannedError` internally.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SpannedError {
    /// The error code
    pub error: Error,
    /// The source span where the error occurred
    pub span: Option<SourceSpan>,
}

impl std::fmt::Display for SpannedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = self.span {
            let span = span.range();
            write!(f, "{} ({}..{})", self.error, span.start, span.end)
        } else {
            write!(f, "{}", self.error)
        }
    }
}

impl std::error::Error for SpannedError {}

// note: this is only used for internal conversions of names
// for now, not the full grammar.
impl From<xee_xpath_ast::ParserError> for Error {
    fn from(e: xee_xpath_ast::ParserError) -> Self {
        let spanned_error: SpannedError = e.into();
        spanned_error.error
    }
}

impl From<xee_xpath_ast::ParserError> for SpannedError {
    fn from(e: xee_xpath_ast::ParserError) -> Self {
        let span = e.span();
        let error = match e {
            ParserError::ExpectedFound { .. } => Error::XPST0003,
            // this is what fn-function-arity-017 expects, even though
            // implementation limit exceeded (XPST00130) seems reasonable to me.
            ParserError::ArityOverflow { .. } => Error::FOAR0002,
            ParserError::Reserved { .. } => Error::XPST0003,
            ParserError::UnknownPrefix { .. } => Error::XPST0081,
            ParserError::UnknownType { .. } => Error::XPST0051,
            // TODO: this this the right error code?
            ParserError::IllegalFunctionInPattern { .. } => Error::XPST0003,
        };
        SpannedError {
            error,
            span: Some(span.into()),
        }
    }
}

impl From<regexml::Error> for Error {
    fn from(e: regexml::Error) -> Self {
        use regexml::Error::*;
        // TODO: pass more error details into error codes
        match e {
            Internal => panic!("Internal error in regexml engine"),
            InvalidFlags(_) => Error::FORX0001,
            Syntax(_) => Error::FORX0002,
            MatchesEmptyString => Error::FORX0003,
            InvalidReplacementString(_) => Error::FORX0004,
        }
    }
}

impl From<xot::Error> for Error {
    fn from(e: xot::Error) -> Self {
        match e {
            xot::Error::MissingPrefix(_) => Error::XPST0081,
            // TODO: are there other xot errors that need to be translated?
            _ => Error::XPST0003,
        }
    }
}

impl From<Error> for SpannedError {
    fn from(e: Error) -> Self {
        SpannedError {
            error: e,
            span: None,
        }
    }
}

// impl From<xee_name::Error> for Error {
//     fn from(e: xee_name::Error) -> Self {
//         match e {
//             xee_name::Error::MissingPrefix => Error::XPST0081,
//         }
//     }
// }

impl From<OutOfBoundsError> for Error {
    fn from(_e: OutOfBoundsError) -> Self {
        Error::FOCA0003
    }
}

/// The result type for errors without span information.
pub type Result<T> = std::result::Result<T, Error>;

/// The result type for errors with (optional) source spans.
///
/// Also known as `SpannedResult` internally.
pub type SpannedResult<T> = std::result::Result<T, SpannedError>;

impl SpannedError {
    /// get the underlying [`Error`] value
    pub fn value(self) -> Error {
        self.error
    }
}
