use xot::{NameId, Node, SpanInfo, SpanInfoKey, Xot};

use crate::{ast_core::Span, error::AttributeError, name::XmlName, names::Names};

/// Parser state affects the parsing output but does not change during parsing.
pub(crate) struct State<'a> {
    pub(crate) xot: &'a mut Xot,
    pub(crate) span_info: SpanInfo,
    pub(crate) names: Names,
}

impl<'a> State<'a> {
    pub(crate) fn new(xot: &'a mut Xot, span_info: SpanInfo, names: Names) -> Self {
        Self {
            xot,
            span_info,
            names,
        }
    }

    pub(crate) fn next(&self, node: Node) -> Option<Node> {
        self.xot.next_sibling(node)
    }

    pub(crate) fn span(&self, node: Node) -> Option<Span> {
        use xot::Value::*;

        match self.xot.value(node) {
            Element(_element) => self.span_info.get(SpanInfoKey::ElementStart(node)),
            Text(_text) => self.span_info.get(SpanInfoKey::Text(node)),
            Comment(_comment) => self.span_info.get(SpanInfoKey::Comment(node)),
            ProcessingInstruction(_pi) => self.span_info.get(SpanInfoKey::PiTarget(node)),
            Document => unreachable!(),
            // TODO: is it worthwhile to introduce span info keys for this?
            Attribute(_attribute) => unreachable!(),
            Namespace(_namespace) => unreachable!(),
        }
        .map(|span| span.into())
    }

    pub(crate) fn attribute_name_span(
        &self,
        node: Node,
        name: NameId,
    ) -> Result<Span, AttributeError> {
        let span = self
            .span_info
            .get(SpanInfoKey::AttributeName(node, name))
            .ok_or(AttributeError::Internal)?;
        Ok(span.into())
    }

    pub(crate) fn attribute_unexpected(
        &self,
        node: Node,
        name: NameId,
        _message: &str,
    ) -> AttributeError {
        let (local, namespace) = self.xot.name_ns_str(name);
        let span = self.attribute_name_span(node, name);
        match span {
            Ok(span) => AttributeError::Unexpected {
                name: XmlName {
                    namespace: namespace.to_string(),
                    local: local.to_string(),
                },
                span,
            },
            Err(e) => e,
        }
    }
}
