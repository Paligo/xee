use xee_xpath::Session;
use xee_xpath_compiler::context::DynamicContext;
use xot::Xot;

use crate::{
    dependency::KnownDependencies,
    environment::Environment,
    renderer::{CharacterRenderer, Renderer, VerboseRenderer},
    testcase::Runnable,
};

pub(crate) struct RunContext<'a> {
    pub(crate) session: Session<'a>,
    pub(crate) known_dependencies: KnownDependencies,
    pub(crate) verbose: bool,
}

impl<'a> RunContext<'a> {
    pub(crate) fn new(
        session: Session<'a>,
        known_dependencies: KnownDependencies,
        verbose: bool,
    ) -> Self {
        Self {
            session,
            known_dependencies,
            verbose,
        }
    }

    pub(crate) fn renderer<E: Environment, R: Runnable<E>>(&self) -> Box<dyn Renderer<E, R>> {
        if self.verbose {
            Box::new(VerboseRenderer::new())
        } else {
            Box::new(CharacterRenderer::new())
        }
    }
}
