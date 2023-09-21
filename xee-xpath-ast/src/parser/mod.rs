mod axis_node_test;
mod kind_test;
mod name;
mod parser_core;
mod primary;
mod signature;
mod types;
mod xpath_type;

use chumsky::input::{SpannedInput, Stream};
use chumsky::{input::ValueInput, prelude::*};
use std::borrow::Cow;

use crate::ast;
use crate::ast::unique_names;
use crate::ast::Span;
use crate::error::{Error, Result};
use crate::lexer::{lexer, Token};
use crate::namespaces::Namespaces;

use super::parser::parser_core::parser;
use super::parser::types::{BoxedParser, State};

fn create_token_iter(src: &str) -> impl Iterator<Item = (Token, SimpleSpan)> + '_ {
    let r = lexer(src).map(|(tok, span)| match tok {
        Ok(tok) => (tok, span.into()),
        Err(()) => (Token::Error, span.into()),
    });
    r
}

fn tokens(src: &str) -> impl ValueInput<'_, Token = Token<'_>, Span = Span> {
    let r = Stream::from_iter(create_token_iter(src)).spanned((src.len()..src.len()).into());
    r
}

// fn tokens2<'src>(
//     src: &'src str,
// ) -> chumsky::input::SpannedInput<
//     Token<'src>,
//     SimpleSpan,
//     chumsky::input::BoxedStream<'src, (Token<'src>, SimpleSpan)>,
// > {
//     let r =
//         Box::new(Stream::from_iter(create_token_iter(src))).spanned((src.len()..src.len()).into());
//     Box::new(r
// }

fn parse<'a, I, T>(
    parser: BoxedParser<'a, I, T>,
    input: I,
    namespaces: Cow<'a, Namespaces<'a>>,
) -> std::result::Result<T, Vec<Rich<'a, Token<'a>>>>
where
    I: ValueInput<'a, Token = Token<'a>, Span = Span>,
    T: std::fmt::Debug,
{
    let mut state = State { namespaces };
    parser.parse_with_state(input, &mut state).into_result()
}

impl ast::XPath {
    pub fn parse<'a>(
        input: &'a str,
        namespaces: &'a Namespaces,
        variables: &'a [ast::Name],
    ) -> Result<'a, Self> {
        let result = parse(parser().xpath, tokens(input), Cow::Borrowed(namespaces));

        match result {
            Ok(mut xpath) => {
                // rename all variables to unique names
                unique_names(&mut xpath, variables);
                Ok(xpath)
            }
            Err(errors) => Err(Error { src: input, errors }),
        }
    }
}

impl ast::ExprSingle {
    pub fn parse(src: &str) -> Result<ast::ExprSingleS> {
        let namespaces = Namespaces::default();
        parse(parser().expr_single, tokens(src), Cow::Owned(namespaces))
            .map_err(|errors| Error { src, errors })
    }
}

impl ast::KindTest {
    pub fn parse(src: &str) -> Result<Self> {
        let namespaces = Namespaces::default();
        parse(parser().kind_test, tokens(src), Cow::Owned(namespaces))
            .map_err(|errors| Error { src, errors })
    }
}

impl ast::Signature {
    pub fn parse<'a>(input: &'a str, namespaces: &'a Namespaces) -> Result<'a, Self> {
        parse(parser().signature, tokens(input), Cow::Borrowed(namespaces))
            .map_err(|errors| Error { src: input, errors })
    }
}

impl ast::SequenceType {
    pub fn parse<'a>(input: &'a str, namespaces: &'a Namespaces) -> Result<'a, ast::SequenceType> {
        parse(
            parser().sequence_type,
            tokens(input),
            Cow::Borrowed(namespaces),
        )
        .map_err(|errors| Error { src: input, errors })
    }
}

impl ast::Name {
    pub fn parse<'a>(src: &'a str, namespaces: &'a Namespaces) -> Result<'a, ast::NameS> {
        parse(parser().name, tokens(src), Cow::Borrowed(namespaces))
            .map_err(|errors| Error { src, errors })
    }
}

struct XPathParserCache;

impl chumsky::cache::Cached for XPathParserCache {
    type Parser<'src> = BoxedParser<
        'src,
        SpannedInput<
            Token<'src>,
            SimpleSpan,
            chumsky::input::BoxedStream<'src, (Token<'src>, SimpleSpan)>,
        >,
        ast::XPath,
    >;

    fn make_parser<'src>(self) -> Self::Parser<'src> {
        Parser::boxed(parser().xpath)
    }
}

struct ExprSingleParserCache;
struct KindTestParserCache;
struct SignatureParserCache;
struct SequenceTypeParserCache;
struct NameParserCache;

pub struct Parsers {
    xpath_parser_cache: chumsky::cache::Cache<XPathParserCache>,
}

impl std::fmt::Debug for Parsers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parsers").finish()
    }
}

impl Parsers {
    pub fn new() -> Self {
        Self {
            xpath_parser_cache: chumsky::cache::Cache::new(XPathParserCache),
        }
    }

    pub fn parse_xpath<'src>(
        &self,
        input: &'src str,
        namespaces: &'src Namespaces,
        variables: &'src [ast::Name],
    ) -> Result<'src, ast::XPath> {
        let mut state = State {
            namespaces: Cow::Borrowed(namespaces),
        };
        let tokens = Stream::from_iter(create_token_iter(input))
            .boxed()
            .spanned((input.len()..input.len()).into());

        let result = self
            .xpath_parser_cache
            .get()
            .parse_with_state(tokens, &mut state)
            .into_result();

        match result {
            Ok(mut xpath) => {
                // rename all variables to unique names
                unique_names(&mut xpath, variables);
                Ok(xpath)
            }
            Err(errors) => Err(Error { src: input, errors }),
        }
    }

    // pub fn parse_expr_single<'a>(input: &'a str) -> Result<'a, ast::ExprSingleS> {}

    // pub fn parse_kind_test<'a>(input: &'a str) -> Result<'a, ast::KindTest> {}

    // pub fn parse_signature<'a>(
    //     input: &'a str,
    //     namespaces: &'a Namespaces,
    // ) -> Result<'a, ast::Signature> {
    // }

    // pub fn parse_sequence_type<'a>(
    //     input: &'a str,
    //     namespaces: &'a Namespaces,
    // ) -> Result<'a, ast::SequenceType> {
    // }

    // pub fn parse_name<'a>(input: &'a str, namespaces: &'a Namespaces) -> Result<'a, ast::NameS> {}
}

#[cfg(test)]
mod tests {
    use crate::FN_NAMESPACE;

    use super::*;

    use insta::assert_ron_snapshot;

    fn parse_xpath_simple(src: &str) -> Result<ast::XPath> {
        let namespaces = Namespaces::default();
        parse(parser().xpath, tokens(src), Cow::Owned(namespaces))
            .map_err(|errors| Error { src, errors })
    }

    fn parse_xpath_simple_element_ns(src: &str) -> Result<ast::XPath> {
        let namespaces = Namespaces::new(Some("http://example.com"), None);
        parse(parser().xpath, tokens(src), Cow::Owned(namespaces))
            .map_err(|errors| Error { src, errors })
    }

    #[test]
    fn test_unprefixed_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Name::parse("foo", &namespaces));
    }

    #[test]
    fn test_prefixed_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Name::parse("xs:foo", &namespaces));
    }

    #[test]
    fn test_qualified_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Name::parse("Q{http://example.com}foo", &namespaces));
    }

    #[test]
    fn test_literal() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1"));
    }

    #[test]
    fn test_var_ref() {
        assert_ron_snapshot!(ast::ExprSingle::parse("$foo"));
    }

    #[test]
    fn test_expr_single_addition() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + 2"));
    }

    #[test]
    fn test_simple_map_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 ! 2"));
    }

    #[test]
    fn test_unary_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("-1"));
    }

    #[test]
    fn test_additive_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + 2"));
    }

    #[test]
    fn test_additive_expr_repeat() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + 2 + 3"));
    }

    #[test]
    fn test_or_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 or 2"));
    }

    #[test]
    fn test_and_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 and 2"));
    }

    #[test]
    fn test_comparison_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 < 2"));
    }

    #[test]
    fn test_concat_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("'a' || 'b'"));
    }

    #[test]
    fn test_nested_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + (2 * 3)"));
    }

    #[test]
    fn test_xpath_single_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + 2"));
    }

    #[test]
    fn test_xpath_multi_expr() {
        assert_ron_snapshot!(parse_xpath_simple("1 + 2, 3 + 4"));
    }

    #[test]
    fn test_single_let_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("let $x := 1 return 5"));
    }

    #[test]
    fn test_single_let_expr_var_ref() {
        assert_ron_snapshot!(ast::ExprSingle::parse("let $x := 1 return $x"));
    }

    #[test]
    fn test_nested_let_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("let $x := 1, $y := 2 return 5"));
    }

    #[test]
    fn test_single_for_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("for $x in 1 return 5"));
    }

    #[test]
    fn test_for_loop() {
        assert_ron_snapshot!(ast::ExprSingle::parse("for $x in 1 to 2 return $x"));
    }

    #[test]
    fn test_if_expr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("if (1) then 2 else 3"));
    }

    #[test]
    fn test_quantified() {
        assert_ron_snapshot!(ast::ExprSingle::parse(
            "every $x in (1, 2) satisfies $x > 0"
        ));
    }

    #[test]
    fn test_quantified_nested() {
        assert_ron_snapshot!(ast::ExprSingle::parse(
            "every $x in (1, 2), $y in (3, 4) satisfies $x > 0 and $y > 0"
        ));
    }

    #[test]
    fn test_inline_function() {
        assert_ron_snapshot!(ast::ExprSingle::parse("function($x) { $x }"));
    }

    #[test]
    fn test_inline_function_with_param_types() {
        assert_ron_snapshot!(ast::ExprSingle::parse("function($x as xs:integer) { $x }"));
    }

    #[test]
    fn test_inline_function_with_return_type() {
        assert_ron_snapshot!(ast::ExprSingle::parse("function($x) as xs:integer { $x }"));
    }

    #[test]
    fn test_inline_function2() {
        assert_ron_snapshot!(ast::ExprSingle::parse("function($x, $y) { $x + $y }"));
    }

    #[test]
    fn test_dynamic_function_call() {
        assert_ron_snapshot!(ast::ExprSingle::parse("$foo()"));
    }

    #[test]
    fn test_dynamic_function_call_args() {
        assert_ron_snapshot!(ast::ExprSingle::parse("$foo(1 + 1, 3)"));
    }

    #[test]
    fn test_dynamic_function_call_placeholder() {
        assert_ron_snapshot!(ast::ExprSingle::parse("$foo(1, ?)"));
    }

    #[test]
    fn test_static_function_call() {
        assert_ron_snapshot!(ast::ExprSingle::parse("my_function()"));
    }

    #[test]
    fn test_static_function_call_fn_prefix() {
        assert_ron_snapshot!(ast::ExprSingle::parse("fn:root()"));
    }

    #[test]
    fn test_static_function_call_q() {
        assert_ron_snapshot!(ast::ExprSingle::parse("Q{http://example.com}something()"));
    }

    #[test]
    fn test_static_function_call_args() {
        assert_ron_snapshot!(ast::ExprSingle::parse("my_function(1, 2)"));
    }

    #[test]
    fn test_named_function_ref() {
        assert_ron_snapshot!(ast::ExprSingle::parse("my_function#2"));
    }

    #[test]
    fn test_static_function_call_placeholder() {
        assert_ron_snapshot!(ast::ExprSingle::parse("my_function(?, 1)"));
    }

    #[test]
    fn test_simple_comma() {
        assert_ron_snapshot!(parse_xpath_simple("1, 2"));
    }

    #[test]
    fn test_complex_comma() {
        assert_ron_snapshot!(parse_xpath_simple("(1, 2), (3, 4)"));
    }

    #[test]
    fn test_range() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 to 2"));
    }

    #[test]
    fn test_simple_map() {
        assert_ron_snapshot!(ast::ExprSingle::parse("(1, 2) ! (. * 2)"));
    }

    #[test]
    fn test_predicate() {
        assert_ron_snapshot!(ast::ExprSingle::parse("(1, 2)[2]"));
    }

    #[test]
    fn test_axis() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo"));
    }

    #[test]
    fn test_multiple_steps() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo/child::bar"));
    }

    #[test]
    fn test_with_predicate() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo[1]"));
    }

    #[test]
    fn test_axis_with_predicate() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo[1]"));
    }

    #[test]
    fn test_axis_star() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::*"));
    }

    #[test]
    fn test_axis_wildcard_prefix() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::*:foo"));
    }

    #[test]
    fn test_axis_wildcard_local_name() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::fn:*"));
    }

    #[test]
    fn test_axis_wildcard_q_name() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::Q{http://example.com}*"));
    }

    #[test]
    fn test_reverse_axis() {
        assert_ron_snapshot!(ast::ExprSingle::parse("parent::foo"));
    }

    #[test]
    fn test_node_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::node()"));
    }

    #[test]
    fn test_text_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::text()"));
    }

    #[test]
    fn test_comment_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::comment()"));
    }

    #[test]
    fn test_namespace_node_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::namespace-node()"));
    }

    #[test]
    fn test_attribute_test_no_args() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::attribute()"));
    }

    #[test]
    fn test_attribute_test_star_arg() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::attribute(*)"));
    }

    #[test]
    fn test_attribute_test_name_arg() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::attribute(foo)"));
    }

    #[test]
    fn test_attribute_test_name_arg_type_arg() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::attribute(foo, bar)"));
    }

    #[test]
    fn test_element_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("self::element()"));
    }

    #[test]
    fn test_abbreviated_forward_step() {
        assert_ron_snapshot!(ast::ExprSingle::parse("foo"));
    }

    #[test]
    fn test_abbreviated_forward_step_with_attribute_test() {
        assert_ron_snapshot!(ast::ExprSingle::parse("foo/attribute()"));
    }

    // XXX should test for attribute axis for SchemaAttributeTest too

    #[test]
    fn test_namespace_node_default_axis() {
        assert_ron_snapshot!(ast::ExprSingle::parse("foo/namespace-node()"));
    }

    #[test]
    fn test_abbreviated_forward_step_attr() {
        assert_ron_snapshot!(ast::ExprSingle::parse("@foo"));
    }

    #[test]
    fn test_abbreviated_reverse_step() {
        assert_ron_snapshot!(ast::ExprSingle::parse("foo/.."));
    }

    #[test]
    fn test_abbreviated_reverse_step_with_predicates() {
        assert_ron_snapshot!(ast::ExprSingle::parse("..[1]"));
    }

    #[test]
    fn test_starts_single_slash() {
        assert_ron_snapshot!(ast::ExprSingle::parse("/child::foo"));
    }

    #[test]
    fn test_single_slash_by_itself() {
        assert_ron_snapshot!(ast::ExprSingle::parse("/"));
    }

    #[test]
    fn test_double_slash_by_itself() {
        assert_ron_snapshot!(ast::ExprSingle::parse("//"));
    }

    #[test]
    fn test_starts_double_slash() {
        assert_ron_snapshot!(ast::ExprSingle::parse("//child::foo"));
    }

    #[test]
    fn test_double_slash_middle() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo//child::bar"));
    }

    #[test]
    fn test_union() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo | child::bar"));
    }

    #[test]
    fn test_intersect() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo intersect child::bar"));
    }

    #[test]
    fn test_except() {
        assert_ron_snapshot!(ast::ExprSingle::parse("child::foo except child::bar"));
    }

    #[test]
    fn test_xpath_parse_error() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 + 2 +"));
    }

    #[test]
    fn test_xpath_ge() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 >= 2"));
    }

    #[test]
    fn test_signature_without_params() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse("fn:foo() as xs:integer", &namespaces));
    }

    #[test]
    fn test_signature_without_params2() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo() as xs:integer*",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_params() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as xs:decimal*) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_node_param() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as node()) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_node_param_and_question_mark() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as node()?) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_minus_in_name() {
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo-bar($a as node()?) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_unary_multiple() {
        assert_ron_snapshot!(ast::ExprSingle::parse("+-1"));
    }

    #[test]
    fn test_cast_as() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 cast as xs:integer"));
    }

    #[test]
    fn test_cast_as_with_question_mark() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 cast as xs:integer?"));
    }

    #[test]
    fn test_castable_as() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 castable as xs:integer"));
    }

    #[test]
    fn test_castable_as_with_question_mark() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 castable as xs:integer?"));
    }

    #[test]
    fn test_instance_of() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 instance of xs:integer"));
    }

    #[test]
    fn test_instance_of_with_star() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 instance of xs:integer*"));
    }

    #[test]
    fn test_treat() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 treat as xs:integer"));
    }

    #[test]
    fn test_treat_with_star() {
        assert_ron_snapshot!(ast::ExprSingle::parse("1 treat as xs:integer*"));
    }

    #[test]
    fn test_default_element_namespace_element_kind_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("element(foo)"));
    }

    #[test]
    fn test_default_element_namespace_attribute_kind_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("attribute(foo)"));
    }

    #[test]
    fn test_default_element_namespace_element_name_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("foo"));
    }

    #[test]
    fn test_default_element_namespace_explicit_element_name_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("child::foo"));
    }

    #[test]
    fn test_default_element_namespace_attribute_name_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("@foo"));
    }

    #[test]
    fn test_default_element_namespace_explicit_attribute_name_test() {
        assert_ron_snapshot!(parse_xpath_simple_element_ns("attribute::foo"));
    }

    #[test]
    fn test_function_call_without_arguments() {
        assert_ron_snapshot!(parse_xpath_simple("fn:foo()"));
    }

    #[test]
    fn test_reserved_function_name() {
        assert_ron_snapshot!(parse_xpath_simple("switch()"));
    }

    #[test]
    fn test_reserved_function_name_reference() {
        assert_ron_snapshot!(parse_xpath_simple("switch#2"));
    }

    #[test]
    fn test_occurrence_indicators_ambiguity() {
        // See Constraint: occurrence-indicators
        assert_ron_snapshot!(parse_xpath_simple("4 treat as item() + - 5"));
    }

    #[test]
    fn test_occurrence_indicators_disambiguate() {
        // See Constraint: occurrence-indicators
        assert_ron_snapshot!(parse_xpath_simple("(4 treat as item()) + - 5"));
    }

    #[test]
    fn test_occurrence_indicators_function() {
        // See Constraint: occurrence-indicators
        assert_ron_snapshot!(parse_xpath_simple("function () as xs:string * {}"));
    }

    #[test]
    fn test_leading_lone_slash_can_form_a_path_expression() {
        // See Constraint: leading-lone-slash

        // if the token immediately following a slash can form a path
        // expression, then the slash must be the beginning of a path
        // expression, not the entirety of it
        assert_ron_snapshot!(parse_xpath_simple("/ *"));
    }

    #[test]
    fn test_leading_lone_slash_can_form_a_path_expression_error() {
        // See Constraint: leading-lone-slash
        assert_ron_snapshot!(parse_xpath_simple("/ * 5"))
    }

    #[test]
    fn test_leading_lone_slash_disambiguate() {
        // See Constraint: leading-lone-slash
        assert_ron_snapshot!(parse_xpath_simple("(/) * 5"))
    }

    #[test]
    fn test_grammar_note_parens() {
        // See Grammar Note: parens
        // This should be interpreted as a comment, not a function call
        assert_ron_snapshot!(parse_xpath_simple("address (: this may be empty :)"));
    }

    #[test]
    fn test_symbol_as_name_test() {
        assert_ron_snapshot!(parse_xpath_simple("/if"))
    }

    #[test]
    fn test_another_symbol_as_name_test() {
        assert_ron_snapshot!(parse_xpath_simple("/else"))
    }

    #[test]
    fn test_symbol_as_name_test_with_prefix() {
        assert_ron_snapshot!(parse_xpath_simple("fn:if"))
    }

    #[test]
    fn test_symbol_as_name_test_with_prefix_wildcard() {
        assert_ron_snapshot!(parse_xpath_simple("*:if"))
    }

    #[test]
    fn test_any_function_type() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::SequenceType::parse("function(*)", &namespaces));
    }

    #[test]
    fn test_typed_function_type() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::SequenceType::parse(
            "function(xs:integer) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_map_constructor() {
        assert_ron_snapshot!(parse_xpath_simple("map { 1: 2 }"))
    }

    #[test]
    fn test_curly_array_constructor() {
        assert_ron_snapshot!(parse_xpath_simple("array { 1, 2}"))
    }

    #[test]
    fn test_square_array_constructor() {
        assert_ron_snapshot!(parse_xpath_simple("[1, 2]"))
    }

    #[test]
    fn test_unary_lookup_name() {
        assert_ron_snapshot!(parse_xpath_simple("?name"))
    }

    #[test]
    fn test_unary_lookup_integer() {
        assert_ron_snapshot!(parse_xpath_simple("?1"))
    }

    #[test]
    fn test_unary_lookup_star() {
        assert_ron_snapshot!(parse_xpath_simple("?*"))
    }

    #[test]
    fn test_unary_lookup_expr() {
        assert_ron_snapshot!(parse_xpath_simple("?(1 + 1)"))
    }

    #[test]
    fn test_lookup_name() {
        assert_ron_snapshot!(parse_xpath_simple("1?name"))
    }

    #[test]
    fn test_any_array() {
        assert_ron_snapshot!(parse_xpath_simple("'foo' instance of array(*)"))
    }

    #[test]
    fn test_typed_array() {
        assert_ron_snapshot!(parse_xpath_simple("'foo' instance of array(xs:integer)"))
    }

    #[test]
    fn test_any_map() {
        assert_ron_snapshot!(parse_xpath_simple("'foo' instance of map(*)"))
    }

    #[test]
    fn test_parse_empty_array() {
        assert_ron_snapshot!(parse_xpath_simple("[]"))
    }

    // #[test]
    // fn test_function_that_takes_function_parameter() {
    //     assert_ron_snapshot!(parse_xpath_simple("filter(1, function($item) { true() })"))
    // }
    // #[test]
    // fn test_symbol_as_name_test_with_localname_wildcard() {
    //     assert_ron_snapshot!(parse_xpath_simple("if:*"))
    // }
}
