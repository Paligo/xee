use crate::{
    environment::{Environment, XPathEnvironmentSpec, XsltEnvironmentSpec},
    ns::{XPATH_TEST_NS, XSLT_TEST_NS},
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

// #[derive(Debug, PartialEq, Eq)]
// pub(crate) struct XsltLanguage;

// impl Language for XsltLanguage {
//     type Environment = XsltEnvironmentSpec;
//     type Runnable: XSLTTestCase;

//     fn catalog_ns() -> &'static str {
//         XSLT_TEST_NS
//     }
// }
