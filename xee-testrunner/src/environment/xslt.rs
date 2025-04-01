use anyhow::Result;

use xee_xpath::{Queries, Query};
use xee_xpath_load::ContextLoadable;

use crate::catalog::LoadContext;

use super::core::{Environment, EnvironmentSpec};

#[derive(Debug, Clone)]
pub(crate) struct Package {
    // TODO
}

#[derive(Debug, Clone)]
pub(crate) struct Stylesheet {
    // TODO
}

#[derive(Debug, Clone)]
pub(crate) struct Output {
    // TODO
}

#[derive(Debug, Clone)]
pub(crate) struct XsltEnvironmentSpec {
    pub(crate) environment_spec: EnvironmentSpec,

    pub(crate) packages: Vec<Package>,
    pub(crate) stylesheets: Vec<Stylesheet>,
    pub(crate) outputs: Vec<Output>,
}

impl XsltEnvironmentSpec {
    pub(crate) fn empty() -> Self {
        Self {
            environment_spec: EnvironmentSpec::empty(),
            packages: vec![],
            stylesheets: vec![],
            outputs: vec![],
        }
    }
}

impl Environment for XsltEnvironmentSpec {
    fn empty() -> Self {
        Self::empty()
    }

    fn environment_spec(&self) -> &EnvironmentSpec {
        &self.environment_spec
    }

    fn load(queries: &Queries, context: &LoadContext) -> Result<impl Query<Self>> {
        let environment_spec_query = EnvironmentSpec::load_with_context(queries, context)?;
        let xslt_environment_spec_query = queries.one(".", move |session, item| {
            Ok(XsltEnvironmentSpec {
                environment_spec: environment_spec_query.execute(session, item)?,
                // TODO
                packages: vec![],
                stylesheets: vec![],
                outputs: vec![],
            })
        })?;
        Ok(xslt_environment_spec_query)
    }
}
