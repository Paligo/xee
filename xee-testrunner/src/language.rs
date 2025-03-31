use crate::{
    environment::{Environment, XPathEnvironmentSpec},
    testcase::{Runnable, XPathTestCase},
};

pub(crate) trait Language: Sized {
    type Environment: Environment;
    type Runnable: Runnable<Self>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct XPathLanguage {}

impl Language for XPathLanguage {
    type Environment = XPathEnvironmentSpec;
    type Runnable = XPathTestCase;
}
