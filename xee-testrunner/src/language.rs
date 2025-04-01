use crate::{
    environment::{Environment, XPathEnvironmentSpec},
    ns::XPATH_TEST_NS,
    testcase::{Runnable, XPathTestCase},
};

pub(crate) trait Language: Sized {
    type Environment: Environment;
    type Runnable: Runnable<Self>;

    fn catalog_ns() -> &'static str;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct XPathLanguage {}

impl Language for XPathLanguage {
    type Environment = XPathEnvironmentSpec;
    type Runnable = XPathTestCase;

    fn catalog_ns() -> &'static str {
        XPATH_TEST_NS
    }
}
