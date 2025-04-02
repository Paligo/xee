use crate::{
    environment::{Environment, XPathEnvironmentSpec, XsltEnvironmentSpec},
    ns::{XPATH_TEST_NS, XSLT_TEST_NS},
    paths::Mode,
    testcase::{Runnable, XPathTestCase, XsltTestCase},
};

pub(crate) trait Language: Sized {
    type Environment: Environment;
    type Runnable: Runnable<Self>;

    fn catalog_ns() -> &'static str;
    fn mode() -> Mode;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct XPathLanguage {}

impl Language for XPathLanguage {
    type Environment = XPathEnvironmentSpec;
    type Runnable = XPathTestCase;

    fn catalog_ns() -> &'static str {
        XPATH_TEST_NS
    }

    fn mode() -> Mode {
        Mode::XPath
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct XsltLanguage;

impl Language for XsltLanguage {
    type Environment = XsltEnvironmentSpec;
    type Runnable = XsltTestCase;

    fn catalog_ns() -> &'static str {
        XSLT_TEST_NS
    }

    fn mode() -> Mode {
        Mode::Xslt
    }
}
