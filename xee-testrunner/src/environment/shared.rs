use anyhow::Result;
use std::path::Path;

use xee_xpath::{context, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::{catalog::LoadContext, hashmap::FxIndexMap, ns::XPATH_TEST_NS};

use super::core::{Environment, EnvironmentRef};

#[derive(Debug, Default, Clone)]
pub(crate) struct SharedEnvironments<E: Environment> {
    environments: FxIndexMap<String, E>,
}

impl<E: Environment> SharedEnvironments<E> {
    pub(crate) fn new(mut environments: FxIndexMap<String, E>) -> Self {
        // there is always an empty environment
        if !environments.contains_key("empty") {
            let empty = E::empty();
            environments.insert("empty".to_string(), empty);
        }
        Self { environments }
    }

    pub(crate) fn get(&self, environment_ref: &EnvironmentRef) -> Option<&E> {
        self.environments.get(&environment_ref.ref_)
    }
}

impl<E: Environment> ContextLoadable<LoadContext> for SharedEnvironments<E> {
    fn static_context_builder<'n>(context: &LoadContext) -> context::StaticContextBuilder<'n> {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(XPATH_TEST_NS);
        builder
    }

    fn load_with_context(
        queries: &Queries,
        context: &LoadContext,
    ) -> Result<impl Query<SharedEnvironments<E>>> {
        let name_query = queries.one("@name/string()", convert_string)?;
        let environment_spec_query = E::load(queries, context)?;
        let environments_query = queries.many("environment", move |session, item| {
            let name = name_query.execute(session, item)?;
            let environment_spec = environment_spec_query.execute(session, item)?;
            Ok((name, environment_spec))
        })?;
        let shared_environments_query = queries.one(".", move |session, item| {
            let environments = environments_query.execute(session, item)?;
            Ok(SharedEnvironments::new(environments.into_iter().collect()))
        })?;
        Ok(shared_environments_query)
    }
}
