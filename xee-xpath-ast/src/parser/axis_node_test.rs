use chumsky::inspector::SimpleState;
use chumsky::{input::ValueInput, prelude::*};

use xee_xpath_lexer::Token;

use crate::ast::Span;
use crate::{ast, error::ParserError};

use super::types::{BoxedParser, State};

#[derive(Clone)]
pub(crate) struct ParserAxisNodeTestOutput<'a, I>
where
    I: ValueInput<'a, Token = Token<'a>, Span = Span>,
{
    pub(crate) axis_node_test: BoxedParser<'a, I, (ast::Axis, ast::NodeTest)>,
}

pub(crate) fn parser_axis_node_test<'a, I>(
    eqname: BoxedParser<'a, I, ast::NameS>,
    kind_test: BoxedParser<'a, I, ast::KindTest>,
) -> ParserAxisNodeTestOutput<'a, I>
where
    I: ValueInput<'a, Token = Token<'a>, Span = Span>,
{
    let local_name_wildcard_token = select! {
        Token::LocalNameWildcard(w) => w
    }
    .boxed();

    let prefix_wildcard_token = select! {
        Token::PrefixWildcard(p) => p
    }
    .boxed();

    let braced_uri_literal_wildcard_token = select! {
        Token::BracedURILiteralWildcard(b) => b
    }
    .boxed();

    let wildcard_prefix =
        local_name_wildcard_token
            .try_map_with(|w, extra| {
                let span = extra.span();
                let state: &mut SimpleState<State> = extra.state();
                let namespace = state.namespaces.by_prefix(w.prefix).ok_or_else(|| {
                    ParserError::UnknownPrefix {
                        prefix: w.prefix.to_string(),
                        span,
                    }
                })?;
                Ok(ast::NameTest::Namespace(namespace.to_string()))
            })
            .boxed();
    let wildcard_braced_uri_literal = braced_uri_literal_wildcard_token
        .map(|w| ast::NameTest::Namespace(w.uri.to_string()))
        .boxed();
    let wildcard_local_name = prefix_wildcard_token
        .map(|w| ast::NameTest::LocalName(w.local_name.to_string()))
        .boxed();
    let wildcard_star = just(Token::Asterisk).to(ast::NameTest::Star).boxed();

    let name_test_wildcard = wildcard_prefix
        .or(wildcard_braced_uri_literal)
        .or(wildcard_local_name)
        .or(wildcard_star)
        .boxed();

    // element names are in the the default element namespace
    let name_test_element_name = eqname
        .clone()
        .map_with(|name, extra| {
            ast::NameTest::Name(name.map(|name| {
                name.with_default_namespace(&extra.state().namespaces.default_element_namespace)
            }))
        })
        .boxed();
    // attribute names are not in the default element namespace
    let name_test_attribute_name = eqname.clone().map(ast::NameTest::Name).boxed();

    // we need to duplicate the sub-parsers for the element and attribute cases
    let name_test_element = name_test_wildcard
        .clone()
        .or(name_test_element_name.clone())
        .boxed();
    let name_test_attribute = name_test_wildcard
        .or(name_test_attribute_name.clone())
        .boxed();

    let parent_axis = just(Token::Parent)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Parent);
    let ancestor_axis = just(Token::Ancestor)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Ancestor);
    let preceding_sibling_axis = just(Token::PrecedingSibling)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::PrecedingSibling);
    let preceding_axis = just(Token::Preceding)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Preceding);
    let ancestor_or_self_axis = just(Token::AncestorOrSelf)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::AncestorOrSelf);

    let reverse_axis = choice::<_>([
        parent_axis,
        ancestor_axis,
        preceding_sibling_axis,
        preceding_axis,
        ancestor_or_self_axis,
    ])
    .boxed();

    let node_test_element_name = kind_test
        .clone()
        .map(ast::NodeTest::KindTest)
        .or(name_test_element.clone().map(ast::NodeTest::NameTest))
        .boxed();
    let node_test_attribute_name = kind_test
        .clone()
        .map(ast::NodeTest::KindTest)
        .or(name_test_attribute.map(ast::NodeTest::NameTest))
        .boxed();

    let abbrev_reverse_step = just(Token::DotDot).to((
        ast::Axis::Parent,
        ast::NodeTest::KindTest(ast::KindTest::Any),
    ));

    // the reverse axis only allows element tests
    let reverse_axis_with_node_test = reverse_axis.then(node_test_element_name.clone()).boxed();
    let reverse_step = reverse_axis_with_node_test.or(abbrev_reverse_step).boxed();

    let child_axis = just(Token::Child)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Child);
    let descendant_axis = just(Token::Descendant)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Descendant);
    let attribute_axis = just(Token::Attribute)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Attribute);
    let self_axis = just(Token::Self_)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Self_);
    let descendant_or_self_axis = just(Token::DescendantOrSelf)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::DescendantOrSelf);
    let following_sibling_axis = just(Token::FollowingSibling)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::FollowingSibling);
    let following_axis = just(Token::Following)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Following);
    let namespace_axis = just(Token::Namespace)
        .ignore_then(just(Token::DoubleColon))
        .to(ast::Axis::Namespace);

    let element_forward_axis = choice::<_>([
        child_axis,
        descendant_axis,
        self_axis,
        descendant_or_self_axis,
        following_sibling_axis,
        following_axis,
        namespace_axis,
    ])
    .boxed();

    let forward_step_with_node_test_element_name = element_forward_axis
        .then(node_test_element_name.clone())
        .boxed();
    let forward_step_with_node_test_attribute_name = attribute_axis
        .then(node_test_attribute_name.clone())
        .boxed();

    let forward_step_with_node_test = forward_step_with_node_test_element_name
        .or(forward_step_with_node_test_attribute_name)
        .boxed();

    let abbrev_forward_step_attribute = just(Token::At).ignore_then(
        node_test_attribute_name
            .clone()
            .map(|node_test| (ast::Axis::Attribute, node_test)),
    );
    let abbrev_forward_step_element = node_test_element_name
        .clone()
        .map(|node_test| {
            // https://www.w3.org/TR/xpath-31/#abbrev
            let axis = match &node_test {
                ast::NodeTest::KindTest(t) => match t {
                    ast::KindTest::Attribute(_) | ast::KindTest::SchemaAttribute(_) => {
                        ast::Axis::Attribute
                    }
                    ast::KindTest::NamespaceNode => ast::Axis::Namespace,
                    _ => ast::Axis::Child,
                },
                _ => ast::Axis::Child,
            };
            (axis, node_test)
        })
        .boxed();

    let abbrev_forward_step = abbrev_forward_step_attribute
        .or(abbrev_forward_step_element)
        .boxed();

    let forward_step = forward_step_with_node_test
        .or(abbrev_forward_step.clone())
        .boxed();

    let axis_node_test = reverse_step.or(forward_step).boxed();

    ParserAxisNodeTestOutput {
        axis_node_test,
    }
}
