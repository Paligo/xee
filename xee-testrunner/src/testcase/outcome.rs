use xee_xpath::error::ErrorValue;

use super::assert::Failure;

#[derive(Debug, PartialEq)]
pub struct UnexpectedError(pub String);

#[derive(Debug, PartialEq)]
pub enum TestOutcome {
    Passed,
    UnexpectedError(UnexpectedError),
    Failed(Failure),
    RuntimeError(ErrorValue),
    CompilationError(ErrorValue),
    UnsupportedExpression(ErrorValue),
    Unsupported,
    EnvironmentError(String),
    Panic,
}

impl TestOutcome {
    pub(crate) fn is_exactly_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
}
