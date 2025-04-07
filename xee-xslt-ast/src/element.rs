use std::sync::OnceLock;

use xee_xpath_ast::XPathParserContext;
use xot::{NameId, Node, Value};

use crate::ast_core::Span;
use crate::ast_core::{self as ast};
use crate::attributes::Attributes;
use crate::combinator::{multi, one, NodeParser, OneParser};
use crate::content::Content;
use crate::context::Context;
use crate::error::ElementError;
use crate::instruction::{DeclarationParser, InstructionParser, SequenceConstructorParser};
use crate::state::State;
use crate::value_template::{ValueTemplateItem, ValueTemplateTokenizer};

pub(crate) type NodeParserLock<V> =
    OnceLock<Box<dyn NodeParser<V> + std::marker::Sync + std::marker::Send>>;

// We use OnceLock to declare content parser once, and then reuse them
pub(crate) type ContentParseLock<V> = OnceLock<
    Box<dyn Fn(&Content) -> Result<V, ElementError> + std::marker::Sync + std::marker::Send>,
>;

static SEQUENCE_CONSTRUCTOR: NodeParserLock<ast::SequenceConstructor> = OnceLock::new();
static SEQUENCE_CONSTRUCTOR_CONTENT: ContentParseLock<ast::SequenceConstructor> = OnceLock::new();
static DECLARATIONS_CONTENT: ContentParseLock<ast::Declarations> = OnceLock::new();

pub(crate) struct XsltParser<'a> {
    state: &'a State,
}

impl<'a> XsltParser<'a> {
    pub(crate) fn new(state: &'a State) -> Self {
        Self { state }
    }

    pub(crate) fn parse_transform(&self, node: Node) -> Result<ast::Transform, ElementError> {
        let parser = instruction(self.state.names.xsl_transform)
            .or(instruction(self.state.names.xsl_stylesheet));
        parser.parse(Some(node), self.state, &Context::empty())
    }
}

impl<'a> Content<'a> {
    pub(crate) fn parse_element<V>(
        self,
        element: &'a xot::Element,
        f: impl FnOnce(&Attributes<'a>) -> Result<V, ElementError>,
    ) -> Result<V, ElementError> {
        let attributes = self.attributes(element).with_standard()?;
        f(&attributes)
    }

    pub(crate) fn span(&self) -> Result<Span, ElementError> {
        self.state.span(self.node).ok_or(ElementError::Internal)
    }

    pub(crate) fn parser_context(&self) -> XPathParserContext {
        self.context.parser_context(self.state)
    }

    pub(crate) fn sequence_constructor(&self) -> Result<ast::SequenceConstructor, ElementError> {
        SEQUENCE_CONSTRUCTOR_CONTENT.get_or_init(|| children(sequence_constructor()))(self)
    }

    pub(crate) fn declarations(&self) -> Result<ast::Declarations, ElementError> {
        DECLARATIONS_CONTENT.get_or_init(|| children(declarations()))(self)
    }
}

pub(crate) fn sequence_constructor() -> impl NodeParser<ast::SequenceConstructor> {
    multi(|content| {
        let node = content.node;
        let state = &content.state;
        let context = &content.context;
        match state.xot.value(node) {
            Value::Text(text) => {
                let span = state.span(node).ok_or(ElementError::Internal)?;
                let parser_context = content.parser_context();
                if context.expand_text {
                    text_value_template(text.get(), span, &parser_context)
                } else {
                    Ok(vec![ast::SequenceConstructorItem::Content(
                        ast::Content::Text(text.get().to_string()),
                    )])
                }
            }
            Value::Element(element) => content.parse_element(element, |attributes| {
                Ok(vec![
                    ast::SequenceConstructorItem::parse_sequence_constructor_item(attributes)?,
                ])
            }),
            _ => Err(ElementError::Unexpected {
                // TODO: get span right
                span: Span::new(0, 0),
            }),
        }
    })
    .flatten()
}

fn declarations() -> impl NodeParser<ast::Declarations> {
    one(|content| {
        match content.state.xot.value(content.node) {
            Value::Element(element) => content.parse_element(element, |attributes| {
                ast::Declaration::parse_declaration(attributes)
            }),
            _ => Err(ElementError::Unexpected {
                // TODO: get span right
                span: Span::new(0, 0),
            }),
        }
    })
    .many()
}

fn text_value_template(
    s: &str,
    span: Span,
    parser_context: &XPathParserContext,
) -> Result<Vec<ast::SequenceConstructorItem>, ElementError> {
    let mut items = Vec::new();
    for token in ValueTemplateTokenizer::new(s, span, parser_context) {
        let token = token?;
        let content = match token {
            ValueTemplateItem::String { text, span: _ } => ast::Content::Text(text.to_string()),
            ValueTemplateItem::Curly { c } => ast::Content::Text(c.to_string()),
            ValueTemplateItem::Value { xpath, span } => {
                ast::Content::Value(Box::new(ast::Expression { xpath, span }))
            }
        };
        let item = ast::SequenceConstructorItem::Content(content);
        items.push(item);
    }
    Ok(items)
}

type ContentParse<V> =
    Box<dyn Fn(&Content) -> Result<V, ElementError> + std::marker::Sync + std::marker::Send>;

pub(crate) fn children<V, P>(parser: P) -> ContentParse<V>
where
    P: NodeParser<V> + std::marker::Sync + std::marker::Send + 'static,
    V: std::marker::Sync + std::marker::Send + 'static,
{
    Box::new(move |content| {
        let (item, next) = parser.parse_next(
            content.state.xot.first_child(content.node),
            content.state,
            &content.context,
        )?;
        // handle end of content check here
        if let Some(next) = next {
            Err(ElementError::Unexpected {
                span: content.state.span(next).ok_or(ElementError::Internal)?,
            })
        } else {
            Ok(item)
        }
    })
}

pub(crate) fn by_element<V>(
    f: impl Fn(&Attributes) -> Result<V, ElementError>,
) -> impl Fn(Content) -> Result<V, ElementError> {
    move |content| {
        let element = content
            .state
            .xot
            .element(content.node)
            .ok_or(ElementError::Unexpected {
                span: content
                    .state
                    .span(content.node)
                    .ok_or(ElementError::Internal)?,
            })?;
        content.parse_element(element, &f)
    }
}

pub(crate) fn by_element_name<V>(
    name: NameId,
    f: impl Fn(&Attributes) -> Result<V, ElementError>,
) -> impl Fn(Content) -> Result<V, ElementError> {
    by_element(move |attributes| {
        if attributes.element.name() == name {
            f(attributes)
        } else {
            Err(ElementError::Unexpected {
                span: attributes.content.span()?,
            })
        }
    })
}

pub(crate) fn by_instruction<V: InstructionParser>(
    name: NameId,
) -> impl Fn(Content) -> Result<V, ElementError> {
    by_element_name(name, move |attributes| V::parse_and_validate(attributes))
}

pub(crate) fn instruction<V: InstructionParser>(
    name: NameId,
) -> OneParser<V, impl Fn(Content) -> Result<V, ElementError>> {
    one(by_instruction(name))
}
