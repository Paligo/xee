mod axis_node_test;
mod kind_test;
mod name;
mod parser_core;
mod pattern;
mod primary;
mod signature;
mod types;
mod xpath_type;

use chumsky::input::Stream;
use chumsky::inspector::SimpleState;
use chumsky::{input::ValueInput, prelude::*};

use std::borrow::Cow;

use xee_name::VariableNames;
use xee_xpath_lexer::{lexer, Token};

use crate::ast;
use crate::ast::unique_names;
use crate::ast::Span;
use crate::error::ParserError;
use crate::Namespaces;

use super::parser::parser_core::parser;
use super::parser::types::{BoxedParser, State};

fn create_token_iter(src: &str) -> impl Iterator<Item = (Token<'_>, SimpleSpan)> + '_ {
    lexer(src).map(|(tok, span)| (tok, span.into()))
}

fn tokens(src: &str) -> impl ValueInput<'_, Token = Token<'_>, Span = Span> {
    Stream::from_iter(create_token_iter(src))
        .map((src.len()..src.len()).into(), |(tok, span)| (tok, span))
}

fn parse<'a, I, T>(
    parser: BoxedParser<'a, I, T>,
    input: I,
    namespaces: Cow<'a, Namespaces>,
) -> std::result::Result<T, ParserError>
where
    I: ValueInput<'a, Token = Token<'a>, Span = Span>,
    T: std::fmt::Debug,
{
    let mut state = SimpleState(State { namespaces });
    parser
        .parse_with_state(input, &mut state)
        .into_result()
        .map_err(|e| e.into_iter().next().unwrap())
}

impl ast::XPath {
    pub fn parse<'a>(
        input: &'a str,
        namespaces: &'a Namespaces,
        variable_names: &'a VariableNames,
    ) -> Result<Self, ParserError> {
        let mut xpath = parse(parser().xpath, tokens(input), Cow::Borrowed(namespaces))?;
        // rename all variables to unique names
        unique_names(&mut xpath, variable_names);
        Ok(xpath)
    }

    // parse xpath, and then a single }
    // This is useful to support XSLT value templates
    pub fn parse_value_template<'a>(
        input: &'a str,
        namespaces: &'a Namespaces,
        variable_names: &'a VariableNames,
    ) -> Result<Self, ParserError> {
        let mut state = SimpleState(State {
            namespaces: Cow::Borrowed(namespaces),
        });
        let r = parser()
            .xpath_right_brace
            .parse_with_state(tokens(input), &mut state);
        let (output, errors) = r.into_output_errors();
        if let Some(mut xpath) = output {
            // rename all variables to unique names
            unique_names(&mut xpath, variable_names);
            Ok(xpath)
        } else {
            Err(errors.into_iter().next().unwrap())
        }
    }
}

impl ast::ExprSingle {
    pub fn parse(src: &str) -> Result<ast::ExprSingleS, ParserError> {
        let namespaces = Namespaces::default();
        parse(parser().expr_single, tokens(src), Cow::Owned(namespaces))
    }
}

impl ast::Signature {
    pub fn parse<'a>(input: &'a str, namespaces: &'a Namespaces) -> Result<Self, ParserError> {
        parse(parser().signature, tokens(input), Cow::Borrowed(namespaces))
    }
}

pub fn parse_kind_test(src: &str) -> Result<ast::KindTest, ParserError> {
    let namespaces = Namespaces::default();
    parse(parser().kind_test, tokens(src), Cow::Owned(namespaces))
}

pub fn parse_sequence_type<'a>(
    input: &'a str,
    namespaces: &'a Namespaces,
) -> Result<ast::SequenceType, ParserError> {
    parse(
        parser().sequence_type,
        tokens(input),
        Cow::Borrowed(namespaces),
    )
}

pub fn parse_item_type<'a>(
    input: &'a str,
    namespaces: &'a Namespaces,
) -> Result<ast::ItemType, ParserError> {
    parse(parser().item_type, tokens(input), Cow::Borrowed(namespaces))
}

pub fn parse_name<'a>(src: &'a str, namespaces: &'a Namespaces) -> Result<ast::NameS, ParserError> {
    parse(parser().name, tokens(src), Cow::Borrowed(namespaces))
}

#[cfg(test)]
mod tests {

    use super::*;

    use ahash::HashSetExt;
    use insta::assert_ron_snapshot;

    fn parse_xpath_simple(src: &str) -> Result<ast::XPath, ParserError> {
        let namespaces = Namespaces::default();
        parse(parser().xpath, tokens(src), Cow::Owned(namespaces))
    }

    fn parse_xpath_simple_element_ns(src: &str) -> Result<ast::XPath, ParserError> {
        let namespaces = Namespaces::new(
            Namespaces::default_namespaces(),
            "http://example.com".to_string(),
            "".to_string(),
        );
        parse(parser().xpath, tokens(src), Cow::Owned(namespaces))
    }

    #[test]
    fn test_unprefixed_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(parse_name("foo", &namespaces));
    }

    #[test]
    fn test_prefixed_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(parse_name("xs:foo", &namespaces));
    }

    #[test]
    fn test_qualified_name() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(parse_name("Q{http://example.com}foo", &namespaces));
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
        assert_ron_snapshot!(ast::ExprSingle::parse("self::attribute(foo, xs:integer)"));
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
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Signature::parse("fn:foo() as xs:integer", &namespaces));
    }

    #[test]
    fn test_signature_without_params2() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo() as xs:integer*",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_params() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as xs:decimal*) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_node_param() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as node()) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_node_param_and_question_mark() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(ast::Signature::parse(
            "fn:foo($a as node()?) as xs:integer",
            &namespaces
        ));
    }

    #[test]
    fn test_signature_with_minus_in_name() {
        let namespaces = Namespaces::default();
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

    // This should be passing, see https://github.com/Paligo/xee/issues/35
    // #[test]
    // fn test_treat_invalid() {
    //     assert_ron_snapshot!(ast::ExprSingle::parse("3 treat as item("));
    // }

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
        assert_ron_snapshot!(parse_sequence_type("function(*)", &namespaces));
    }

    #[test]
    fn test_typed_function_type() {
        let namespaces = Namespaces::default();
        assert_ron_snapshot!(parse_sequence_type(
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

    #[test]
    fn test_arrow_function_static() {
        assert_ron_snapshot!(parse_xpath_simple("'foo' => fn:concat('bar')"))
    }

    #[test]
    fn test_arrow_function_static_with_placeholder() {
        assert_ron_snapshot!(parse_xpath_simple("'$' => fn:concat(?)"))
    }

    #[test]
    fn test_inline_function_with_empty_body() {
        assert_ron_snapshot!(parse_xpath_simple(
            "let $f := function($x) { (: there's nothing here :)} return $f(2)"
        ))
    }

    #[test]
    fn test_comment_before_expression() {
        assert_ron_snapshot!(parse_xpath_simple("(: comment :)/foo"))
    }

    #[test]
    fn test_multiple_comments_before_expression() {
        assert_ron_snapshot!(parse_xpath_simple(r#"(: a :)(: b :)foo"#))
    }
    #[test]
    fn test_xpath_parse_value_template() {
        let namespaces = Namespaces::default();
        let xpath =
            ast::XPath::parse_value_template("1 + 2}", &namespaces, &VariableNames::new()).unwrap();
        assert_eq!(xpath.0.span, Span::new(0, 5));
        assert_ron_snapshot!(xpath);
    }

    #[test]
    fn test_xpath_parse_value_template_with_leftover() {
        let namespaces = Namespaces::default();
        let xpath =
            ast::XPath::parse_value_template("1 + 2}foo", &namespaces, &VariableNames::new())
                .unwrap();
        assert_eq!(xpath.0.span, Span::new(0, 5));
        assert_ron_snapshot!(xpath);
    }

    #[test]
    fn test_xpath_parse_value_template_a_with_leftover() {
        let namespaces = Namespaces::default();
        let xpath =
            ast::XPath::parse_value_template("a}foo", &namespaces, &VariableNames::new()).unwrap();
        assert_eq!(xpath.0.span, Span::new(0, 1));
        assert_ron_snapshot!(xpath);
    }

    #[test]
    fn test_xpath_parse_value_template_with_second_value_following() {
        let namespaces = Namespaces::default();
        let xpath =
            ast::XPath::parse_value_template("a}foo{b}!", &namespaces, &VariableNames::new())
                .unwrap();
        assert_eq!(xpath.0.span, Span::new(0, 1));
        assert_ron_snapshot!(xpath);
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
