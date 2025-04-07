use xee_xpath_compiler::context::Variables;
use xot::Xot;

use crate::ast_core as ast;
use crate::error::ElementError as Error;
use crate::instruction::SequenceConstructorParser;
use crate::staticeval::static_evaluate;
use crate::{content::Content, context::Context, element::XsltParser, names::Names, state::State};

type Result<V> = std::result::Result<V, Error>;

pub fn parse_transform(s: &str) -> Result<ast::Transform> {
    let mut xot = Xot::new();
    let names = Names::new(&mut xot);
    let (node, span_info) = xot
        .parse_with_span_info(s)
        .map_err(|_e| Error::Unsupported)?;
    let node = xot.document_element(node).unwrap();
    let mut state = State::new(xot, span_info, names);

    let mut xot = Xot::new();
    static_evaluate(&mut state, node, Variables::new(), &mut xot)
        .map_err(|_e| Error::Unsupported)?;
    let parser = XsltParser::new(&state);
    parser.parse_transform(node)
}

pub fn parse_sequence_constructor_item(s: &str) -> Result<ast::SequenceConstructorItem> {
    let mut xot = Xot::new();
    let names = Names::new(&mut xot);
    let (node, span_info) = xot.parse_with_span_info(s).unwrap();
    let state = State::new(xot, span_info, names);
    let node = state.xot.document_element(node).unwrap();

    if let Some(element) = state.xot.element(node) {
        let context = Context::new(state.xot.prefixes(node));
        let content = Content::new(node, &state, context);
        content.parse_element(
            element,
            ast::SequenceConstructorItem::parse_sequence_constructor_item,
        )
    } else {
        Err(Error::Internal)
    }
}
