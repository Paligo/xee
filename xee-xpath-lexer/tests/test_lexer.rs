use std::borrow::Cow;

use ibig::ibig;
use rust_decimal_macros::dec;
use xee_xpath_lexer::{lexer, PrefixWildcard, PrefixedQName, Token};

#[test]
fn test_tokenize() {
    let mut lex = lexer("cast as");
    assert_eq!(lex.next(), Some((Token::Cast, (0..4))));
    assert_eq!(lex.next(), Some((Token::As, (5..7))));
}

#[test]
fn test_comment_by_itself() {
    let mut lex = lexer("(: this is a comment :)");
    assert_eq!(lex.next(), None);
}

#[test]
fn test_comment_weird_content() {
    let mut lex = lexer("(: 1name :)");
    assert_eq!(lex.next(), None);
}

#[test]
fn test_comment_multiline() {
    let mut lex = lexer("(: this is\na comment :)");
    assert_eq!(lex.next(), None);
}

#[test]
fn test_comment_nested() {
    let mut lex = lexer("(: this (:is a:) comment :)");
    assert_eq!(lex.next(), None);
}

#[test]
fn test_comment_nested_broken() {
    let mut lex = lexer("(: this (:is a:) comment :");
    assert_eq!(lex.next(), Some((Token::Error, (0..26))));
}

#[test]
fn test_close_comment_by_itself() {
    let mut lex = lexer(":)");
    assert_eq!(lex.next(), Some((Token::Colon, (0..1))));
    assert_eq!(lex.next(), Some((Token::RightParen, (1..2))));
}

#[test]
fn test_open_comment_by_itself() {
    let mut lex = lexer("(:");
    // this is an unterminated comment
    assert_eq!(lex.next(), Some((Token::Error, (0..2))));
}

#[test]
fn test_integer_literal() {
    let mut lex = lexer("123");
    assert_eq!(
        lex.next(),
        Some((Token::IntegerLiteral(ibig!(123)), (0..3)))
    );
}

#[test]
fn test_decimal_literal() {
    let mut lex = lexer("123.456");
    assert_eq!(
        lex.next(),
        Some((Token::DecimalLiteral(dec!(123.456)), (0..7)))
    );
}

#[test]
fn test_decimal_starts_with_dot() {
    let mut lex = lexer(".456");
    assert_eq!(
        lex.next(),
        Some((Token::DecimalLiteral(dec!(0.456)), (0..4)))
    );
}

#[test]
fn test_decimal_literal_too_big() {
    let mut lex = lexer("12300000000000000000000000000000000.456");
    assert_eq!(lex.next(), Some((Token::Error, (0..39))));
}

#[test]
fn test_double_literal() {
    let mut lex = lexer("123.456e-5");
    assert_eq!(
        lex.next(),
        Some((Token::DoubleLiteral(123.456e-5), (0..10)))
    );
}

#[test]
fn test_double_literal_starts_with_dot() {
    let mut lex = lexer(".456e-5");
    assert_eq!(lex.next(), Some((Token::DoubleLiteral(0.456e-5), (0..7))));
}

#[test]
fn test_string_literal_double_quotes() {
    let mut lex = lexer(r#""foo""#);
    assert_eq!(
        lex.next(),
        Some((Token::StringLiteral(Cow::Borrowed("foo")), (0..5)))
    );
}

#[test]
fn test_string_literal_double_quotes_escape() {
    let mut lex = lexer(r#""fo""o""#);
    assert_eq!(
        lex.next(),
        Some((
            Token::StringLiteral(Cow::Owned(r#"fo"o"#.to_string())),
            (0..7)
        ))
    );
}

#[test]
fn test_string_literal_single_quotes() {
    let mut lex = lexer(r#"'foo'"#);
    assert_eq!(
        lex.next(),
        Some((Token::StringLiteral(Cow::Borrowed("foo")), (0..5)))
    );
}

#[test]
fn test_string_literal_single_quotes_escape() {
    let mut lex = lexer(r#"'fo''o'"#);
    assert_eq!(
        lex.next(),
        Some((
            Token::StringLiteral(Cow::Owned(r#"fo'o"#.to_string())),
            (0..7)
        ))
    );
}

#[test]
fn test_string_literal_single_quotes_escape2() {
    let mut lex = lexer(r#"'fo''o''l'"#);
    assert_eq!(
        lex.next(),
        Some((
            Token::StringLiteral(Cow::Owned(r#"fo'o'l"#.to_string())),
            (0..10)
        ))
    );
}

#[test]
fn test_ncname() {
    let mut lex = lexer("foo");
    assert_eq!(lex.next(), Some((Token::NCName("foo"), (0..3))));
}

#[test]
fn test_prefixed_name() {
    let mut lex = lexer("xs:integer");
    assert_eq!(
        lex.next(),
        Some((
            (Token::PrefixedQName(PrefixedQName {
                prefix: "xs",
                local_name: "integer"
            })),
            (0..10)
        ))
    );
}

#[test]
fn test_axis_lexer() {
    let mut lex = lexer("ancestor::foo");
    assert_eq!(lex.next(), Some((Token::Ancestor, (0..8))));
    assert_eq!(lex.next(), Some((Token::DoubleColon, (8..10))));
    assert_eq!(lex.next(), Some((Token::NCName("foo"), (10..13))));
}

#[test]
fn test_braced_uri_literal() {
    let mut lex = lexer("Q{http://example.com}");
    assert_eq!(
        lex.next(),
        Some((Token::BracedURILiteral("http://example.com"), (0..21)))
    );
}

#[test]
fn two_non_delimiting_tokens_not_separated() {
    let mut lex = lexer("1cast");
    assert_eq!(lex.next(), Some((Token::Error, 0..1)));
    assert_eq!(lex.next(), Some((Token::Cast, 1..5)));
}

#[test]
fn delimiting_and_non_delimiting_not_separated() {
    let mut lex = lexer("'1'cast");
    assert_eq!(
        lex.next(),
        Some((Token::StringLiteral(Cow::Borrowed("1")), (0..3)))
    );
    assert_eq!(lex.next(), Some((Token::Cast, 3..7)));
}

#[test]
fn non_delimiting_and_delimiting_not_separated() {
    let mut lex = lexer("cast'1'");
    assert_eq!(lex.next(), Some((Token::Cast, 0..4)));
    assert_eq!(
        lex.next(),
        Some((Token::StringLiteral(Cow::Borrowed("1")), (4..7)))
    );
}

#[test]
fn two_non_delimiting_tokens_separated_by_whitespace() {
    let mut lex = lexer("1 cast");
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(1)), (0..1))));
    assert_eq!(lex.next(), Some((Token::Cast, 2..6)));
}

#[test]
fn two_non_delimiting_tokens_separated_by_comment() {
    let mut lex = lexer("1(:hello:)cast");
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(1)), (0..1))));
    assert_eq!(lex.next(), Some((Token::Cast, (10..14))));
}

#[test]
fn test_comment_in_middle_of_integer() {
    let mut lex = lexer("1(:hello:)2");
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(1)), (0..1))));
    assert_eq!(
        lex.next(),
        Some((Token::IntegerLiteral(ibig!(2)), (10..11)))
    );
}

#[test]
fn comment_after_prefix_after_colon() {
    let mut lex = lexer("foo:(:hey:)ncname");
    assert_eq!(lex.next(), Some((Token::NCName("foo"), 0..3)));
    assert_eq!(lex.next(), Some((Token::Colon, 3..4)));
    assert_eq!(lex.next(), Some((Token::NCName("ncname"), 11..17)));
}

#[test]
fn comment_in_uri_qualified_name() {
    let mut lex = lexer("Q{foo}(:hey:)ncname");
    assert_eq!(lex.next(), Some((Token::BracedURILiteral("foo"), (0..6))));
    assert_eq!(lex.next(), Some((Token::NCName("ncname"), 13..19)));
}

#[test]
fn comment_in_braced_uri_literal() {
    let mut lex = lexer("Q{(:hey:)foo}");
    assert_eq!(
        lex.next(),
        Some((Token::BracedURILiteral("(:hey:)foo"), (0..13)))
    );
}

#[test]
fn comment_after_wildcard_after_colon() {
    let mut lex = lexer("*:(:hey:)ncname");
    assert_eq!(lex.next(), Some((Token::AsteriskColon, 0..2)));
    assert_eq!(lex.next(), Some((Token::NCName("ncname"), 9..15)));
}

#[test]
fn comment_after_wildcard_before_colon() {
    let mut lex = lexer("name(:hey:):*");
    assert_eq!(lex.next(), Some((Token::NCName("name"), 0..4)));
    assert_eq!(lex.next(), Some((Token::ColonAsterisk, 11..13)));
}

#[test]
fn comment_with_quote() {
    let mut lex = lexer("(: there's nothing here :)");
    assert_eq!(lex.next(), None);
}

#[test]
fn multiple_comments() {
    let mut lex = lexer("(: a :)(: b :)foo");
    assert_eq!(lex.next(), Some((Token::NCName("foo"), 14..17)));
}

#[test]
fn whitespace_between_prefix_and_wildcard() {
    let mut lex = lexer("ncname :*");
    assert_eq!(lex.next(), Some((Token::NCName("ncname"), 0..6)));
    assert_eq!(lex.next(), Some((Token::ColonAsterisk, 7..9)));
}

#[test]
fn qname_then_dot_is_ncname() {
    let mut lex = lexer("xs:integer.");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "xs",
                local_name: "integer."
            }),
            0..11
        ))
    );
    assert_eq!(lex.next(), None);
}

#[test]
fn qname_then_dot_with_separator_is_ok() {
    let mut lex = lexer("xs:integer .");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "xs",
                local_name: "integer"
            }),
            (0..10)
        ))
    );
    assert_eq!(lex.next(), Some((Token::Dot, 11..12)));
}

#[test]
fn qname_then_minus_is_ncname() {
    let mut lex = lexer("xs:integer-");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "xs",
                local_name: "integer-"
            }),
            (0..11)
        ))
    );
    assert_eq!(lex.next(), None);
}

#[test]
fn qname_then_minus_with_separator_is_ok() {
    let mut lex = lexer("xs:integer -");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "xs",
                local_name: "integer"
            }),
            (0..10)
        ))
    );
    assert_eq!(lex.next(), Some((Token::Minus, 11..12)));
}

#[test]
fn decimal_with_dot_must_have_separator() {
    let mut lex = lexer("1.0.");
    assert_eq!(lex.next(), Some((Token::Error, (0..3))));
    assert_eq!(lex.next(), Some((Token::Dot, 3..4)));
}

#[test]
fn double_with_dot_must_have_separator() {
    let mut lex = lexer("1.2e3.");
    assert_eq!(lex.next(), Some((Token::Error, 0..5)));
    assert_eq!(lex.next(), Some((Token::Dot, 5..6)));
}

#[test]
fn dot_with_decimal_must_have_separator() {
    let mut lex = lexer(".1.2");
    assert_eq!(lex.next(), Some((Token::Error, 0..2)));
    assert_eq!(lex.next(), Some((Token::DecimalLiteral(dec!(0.2)), 2..4)));
}

#[test]
fn test_simple_map() {
    let mut lex = lexer("(1, 2) ! (. * 2)");
    assert_eq!(lex.next(), Some((Token::LeftParen, (0..1))));
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(1)), (1..2))));
    assert_eq!(lex.next(), Some((Token::Comma, (2..3))));
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(2)), (4..5))));
    assert_eq!(lex.next(), Some((Token::RightParen, (5..6))));
    assert_eq!(lex.next(), Some((Token::ExclamationMark, (7..8))));
    assert_eq!(lex.next(), Some((Token::LeftParen, (9..10))));
    assert_eq!(lex.next(), Some((Token::Dot, (10..11))));
    assert_eq!(lex.next(), Some((Token::Asterisk, (12..13))));
    assert_eq!(
        lex.next(),
        Some((Token::IntegerLiteral(ibig!(2)), (14..15)))
    );
    assert_eq!(lex.next(), Some((Token::RightParen, (15..16))));
}

#[test]
fn test_ncname_contains_minus() {
    let mut lex = lexer("a-b");
    assert_eq!(lex.next(), Some((Token::NCName("a-b"), (0..3))));
}

#[test]
fn test_fn_if_is_a_qname() {
    let mut lex = lexer("fn:if");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "fn",
                local_name: "if"
            }),
            (0..5)
        ))
    );
}

#[test]
fn test_array_map_is_a_qname() {
    let mut lex = lexer("array:map");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "array",
                local_name: "map"
            }),
            (0..9)
        ))
    );
}

#[test]
fn test_prefix_wildcard() {
    let mut lex = lexer("*:if");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixWildcard(PrefixWildcard { local_name: "if" }),
            (0..4)
        ))
    );
}

#[test]
fn test_reserved() {
    let mut lex = lexer("map()");
    assert_eq!(lex.next(), Some((Token::Map, (0..3))));
    assert_eq!(lex.next(), Some((Token::LeftParen, (3..4))));
    assert_eq!(lex.next(), Some((Token::RightParen, (4..5))));
}

#[test]
fn test_reserved_switch() {
    let mut lex = lexer("switch()");
    assert_eq!(lex.next(), Some((Token::Switch, (0..6))));
    assert_eq!(lex.next(), Some((Token::LeftParen, (6..7))));
    assert_eq!(lex.next(), Some((Token::RightParen, (7..8))));
}

#[test]
fn test_reserved_duplicated() {
    let mut lex = lexer("mapmap");
    assert_eq!(lex.next(), Some((Token::NCName("mapmap"), (0..6))));
}

#[test]
fn test_map_constructor_025() {
    let mut lex = lexer("map{$m?a:true()}");
    assert_eq!(lex.next(), Some((Token::Map, (0..3))));
    assert_eq!(lex.next(), Some((Token::LeftBrace, (3..4))));
    assert_eq!(lex.next(), Some((Token::Dollar, (4..5))));
    assert_eq!(lex.next(), Some((Token::NCName("m"), (5..6))));
    assert_eq!(lex.next(), Some((Token::QuestionMark, (6..7))));
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "a",
                local_name: "true"
            }),
            (7..13)
        ))
    );
    assert_eq!(lex.next(), Some((Token::LeftParen, (13..14))));
    assert_eq!(lex.next(), Some((Token::RightParen, (14..15))));
}

#[test]
fn test_function_name_026() {
    let mut lex = lexer("fn:function-name(fn:lang#1)");
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "fn",
                local_name: "function-name"
            }),
            (0..16)
        ))
    );
    assert_eq!(lex.next(), Some((Token::LeftParen, (16..17))));
    assert_eq!(
        lex.next(),
        Some((
            Token::PrefixedQName(PrefixedQName {
                prefix: "fn",
                local_name: "lang"
            }),
            (17..24)
        ))
    );
    assert_eq!(lex.next(), Some((Token::Hash, (24..25))));
    assert_eq!(
        lex.next(),
        Some((Token::IntegerLiteral(ibig!(1)), (25..26)))
    );
    assert_eq!(lex.next(), Some((Token::RightParen, (26..27))));
    assert_eq!(lex.next(), None);
}

#[test]
fn test_text_predicate_tokens() {
    let mut lex = lexer("text()[1]");
    assert_eq!(lex.next(), Some((Token::Text, (0..4))));
    assert_eq!(lex.next(), Some((Token::LeftParen, (4..5))));
    assert_eq!(lex.next(), Some((Token::RightParen, (5..6))));
    assert_eq!(lex.next(), Some((Token::LeftBracket, (6..7))));
    assert_eq!(lex.next(), Some((Token::IntegerLiteral(ibig!(1)), (7..8))));
    assert_eq!(lex.next(), Some((Token::RightBracket, (8..9))));
    assert_eq!(lex.next(), None);
}
