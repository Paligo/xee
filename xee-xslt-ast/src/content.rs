use xot::Node;

use crate::attributes::Attributes;
use crate::context::Context;
use crate::state::State;

#[derive(Clone)]
pub(crate) struct Content<'a> {
    pub(crate) node: Node,
    pub(crate) state: &'a State<'a>,
    pub(crate) context: Context,
}

impl<'a> Content<'a> {
    pub(crate) fn new(node: Node, state: &'a State, context: Context) -> Self {
        Self {
            node,
            state,
            context,
        }
    }

    pub(crate) fn with_context(&self, context: Context) -> Self {
        Self {
            context,
            ..self.clone()
        }
    }

    pub(crate) fn with_node(&self, node: Node) -> Self {
        Self {
            node,
            ..self.clone()
        }
    }

    pub(crate) fn attributes(self, element: &'a xot::Element) -> Attributes<'a> {
        Attributes::new(self, element)
    }

    pub(crate) fn xot_namespaces(&self) -> xot::Namespaces<'_> {
        self.state.xot.namespaces(self.node)
    }

    pub(crate) fn xot_attributes(&self) -> xot::Attributes<'a> {
        self.state.xot.attributes(self.node)
    }
}
