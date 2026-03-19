use itertools::{Itertools, MultiPeek};
#[cfg(test)]
use logos::Logos;
use logos::{Span, SpannedIter};

use crate::{
    lexer::{
        BracedURILiteralWildcard, LocalNameWildcard, PrefixWildcard, PrefixedQName,
        URIQualifiedName,
    },
    Token,
};

// this iterator is used to combine multiple tokens into a prefixed qname, uri qualified name,
// prefix wildcard, local name wildcard, uri qualified name wildcard.
// these tokens cannot have whitespace inside, but by the time tokens reach the
// parser all whitespace has been elimited, so we cannot use the parser to enforce this.
// we therefore combine them early here, where we can still see the whitespace.
// this iterator also turns any errors into error tokens
pub(crate) struct ExplicitWhitespace<'a> {
    base: MultiPeek<SpannedIter<'a, Token<'a>>>,
}

impl<'a> ExplicitWhitespace<'a> {
    pub(crate) fn new(spanned_iter: SpannedIter<'a, Token<'a>>) -> Self {
        let base = spanned_iter.multipeek();
        Self { base }
    }

    #[cfg(test)]
    pub(crate) fn from_str(input: &'a str) -> Self {
        let spanned_lexer = Token::lexer(input).spanned();
        Self::new(spanned_lexer)
    }

    // if we find a qname, this could be merged into a prefixed qname, or a
    // local name wildcard.
    fn qname_prefix<'b>(&mut self, name: &'a str, span: &'b Span) -> Option<(Token<'a>, Span)> {
        let (next_token, next_span) = self.base.peek()?;

        match next_token {
            // if we are followed by a colon token, this may be a prefixed qname
            Ok(Token::Colon) => {
                // and then something that can be interpreted as a local name;
                // either a ncname or a reserved name
                if let Some((Ok(local_name_token), local_name_span)) = self.base.peek() {
                    if let Some(local_name) = local_name_token.ncname() {
                        // we create a span from the start of the original prefix span
                        // to the end of the localname span
                        let span = span.start..local_name_span.end;
                        return Some((
                            Token::PrefixedQName(PrefixedQName {
                                prefix: name,
                                local_name,
                            }),
                            span,
                        ));
                    }
                }
                None
            }
            // if we are followed by a ColonAsterisk token, this is a local name wildcard
            Ok(Token::ColonAsterisk) => {
                let span = span.start..next_span.end;
                Some((
                    Token::LocalNameWildcard(LocalNameWildcard { prefix: name }),
                    span,
                ))
            }
            Ok(_) => None,
            Err(_) => Some((Token::Error, span.clone())),
        }
    }

    fn braced_uri_literal_prefix<'b>(
        &mut self,
        uri: &'a str,
        span: &'b Span,
    ) -> Option<(Token<'a>, Span)> {
        let (next_token, next_span) = self.base.peek()?;
        match next_token {
            Ok(Token::NCName(local_name)) => {
                let span = span.start..next_span.end;
                Some((
                    Token::URIQualifiedName(URIQualifiedName { uri, local_name }),
                    span,
                ))
            }
            Ok(Token::Asterisk) => {
                let span = span.start..next_span.end;
                Some((
                    Token::BracedURILiteralWildcard(BracedURILiteralWildcard { uri }),
                    span,
                ))
            }
            Err(_) => Some((Token::Error, span.clone())),
            _ => None,
        }
    }

    fn prefix_wildcard(&mut self, span: &Span) -> Option<(Token<'a>, Span)> {
        let (next_token, next_span) = self.base.peek()?;
        match next_token {
            Ok(next_token) => {
                if let Some(local_name) = next_token.ncname() {
                    let span = span.start..next_span.end;
                    Some((Token::PrefixWildcard(PrefixWildcard { local_name }), span))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl<'a> Iterator for ExplicitWhitespace<'a> {
    type Item = (Token<'a>, Span);

    fn next(&mut self) -> Option<Self::Item> {
        let (token, span) = self.base.next()?;
        if let Ok(token) = token {
            // if we have something that is a valid ncname, it may be merged into either
            // a prefixed qname or local name wildcard
            if let Some(name) = token.ncname() {
                if let Some((token, span)) = self.qname_prefix(name, &span) {
                    match token {
                        Token::PrefixedQName(_) => {
                            // consume two next tokens
                            self.base.next();
                            self.base.next();
                        }
                        Token::LocalNameWildcard(_) => {
                            // consume next token
                            self.base.next();
                        }
                        _ => {}
                    }
                    return Some((token, span));
                }
            }
            match token {
                // if we have asterisk colon it may be merged into a prefix wildcard
                Token::AsteriskColon => {
                    if let Some((token, span)) = self.prefix_wildcard(&span) {
                        // consume next token
                        self.base.next();
                        return Some((token, span));
                    }
                    Some((token, span))
                }
                // if we have a braced URI literal it may be either a URI qualified name
                // or a braced uri literal wildcard
                Token::BracedURILiteral(uri) => {
                    if let Some((token, span)) = self.braced_uri_literal_prefix(uri, &span) {
                        // consume next token
                        self.base.next();
                        return Some((token, span));
                    }
                    Some((token, span))
                }
                _ => Some((token, span)),
            }
        } else {
            Some((Token::Error, span))
        }
    }
}

#[cfg(test)]
mod tests {
    use ibig::ibig;

    use super::*;

    #[test]
    fn test_no_ncname_no_prefixed_qname() {
        let mut iter = ExplicitWhitespace::from_str("1 + 1");
        assert_eq!(iter.next(), Some((Token::IntegerLiteral(ibig!(1)), 0..1)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 1..2)));
        assert_eq!(iter.next(), Some((Token::Plus, 2..3)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 3..4)));
        assert_eq!(iter.next(), Some((Token::IntegerLiteral(ibig!(1)), 4..5)));
    }

    #[test]
    fn test_ncname_no_prefixed_qname() {
        let mut iter = ExplicitWhitespace::from_str("foo + 1");
        assert_eq!(iter.next(), Some((Token::NCName("foo"), 0..3)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 3..4)));
        assert_eq!(iter.next(), Some((Token::Plus, 4..5)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 5..6)));
        assert_eq!(iter.next(), Some((Token::IntegerLiteral(ibig!(1)), 6..7)));
    }

    #[test]
    fn test_ncname_colon_no_prefixed_qname() {
        let mut iter = ExplicitWhitespace::from_str("foo: + 1");
        assert_eq!(iter.next(), Some((Token::NCName("foo"), 0..3)));
        assert_eq!(iter.next(), Some((Token::Colon, 3..4)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 4..5)));
        assert_eq!(iter.next(), Some((Token::Plus, 5..6)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 6..7)));
        assert_eq!(iter.next(), Some((Token::IntegerLiteral(ibig!(1)), 7..8)));
    }

    #[test]
    fn test_prefixed_qname() {
        let mut iter = ExplicitWhitespace::from_str("foo:bar");
        assert_eq!(
            iter.next(),
            Some((
                Token::PrefixedQName(PrefixedQName {
                    prefix: "foo",
                    local_name: "bar"
                }),
                0..7
            ))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_prefixed_qname_followed() {
        let mut iter = ExplicitWhitespace::from_str("foo:bar + 1");
        assert_eq!(
            iter.next(),
            Some((
                Token::PrefixedQName(PrefixedQName {
                    prefix: "foo",
                    local_name: "bar"
                }),
                0..7
            ))
        );
        assert_eq!(iter.next(), Some((Token::Whitespace, 7..8)));
        assert_eq!(iter.next(), Some((Token::Plus, 8..9)));
        assert_eq!(iter.next(), Some((Token::Whitespace, 9..10)));
        assert_eq!(iter.next(), Some((Token::IntegerLiteral(ibig!(1)), 10..11)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_local_name_wildcard() {
        let mut iter = ExplicitWhitespace::from_str("foo:*");
        assert_eq!(
            iter.next(),
            Some((
                Token::LocalNameWildcard(LocalNameWildcard { prefix: "foo" }),
                0..5
            ))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_prefix_wilcard() {
        let mut iter = ExplicitWhitespace::from_str("*:bar");
        assert_eq!(
            iter.next(),
            Some((
                Token::PrefixWildcard(PrefixWildcard { local_name: "bar" }),
                0..5
            ))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_uri_qualified_name() {
        let mut iter = ExplicitWhitespace::from_str("Q{http://example.com}bar");
        assert_eq!(
            iter.next(),
            Some((
                Token::URIQualifiedName(URIQualifiedName {
                    uri: "http://example.com",
                    local_name: "bar"
                }),
                0..24
            ))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_braced_uri_literal_wildcard() {
        let mut iter = ExplicitWhitespace::from_str("Q{http://example.com}*");
        assert_eq!(
            iter.next(),
            Some((
                Token::BracedURILiteralWildcard(BracedURILiteralWildcard {
                    uri: "http://example.com"
                }),
                0..22
            ))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_comment_in_uri_qualified_name() {
        // there is a comment in here, which is in error
        let mut iter = ExplicitWhitespace::from_str("Q{foo}(:hey:)ncname");
        assert_eq!(iter.next(), Some((Token::BracedURILiteral("foo"), 0..6)));
        assert_eq!(iter.next(), Some((Token::CommentStart, 6..13)));
        assert_eq!(iter.next(), Some((Token::NCName("ncname"), 13..19)));
    }

    // Axis names and other keywords must be valid as local names in QNames.
    // For example, "ex:child" must produce a PrefixedQName, not separate tokens.
    #[test]
    fn test_prefixed_qname_with_axis_name_local_names() {
        let cases = [
            ("ex:child", "child", 8),
            ("ex:parent", "parent", 9),
            ("ex:self", "self", 7),
            ("ex:attribute", "attribute", 12),
            ("ex:descendant", "descendant", 13),
            ("ex:descendant-or-self", "descendant-or-self", 21),
            ("ex:ancestor", "ancestor", 11),
            ("ex:ancestor-or-self", "ancestor-or-self", 19),
            ("ex:following", "following", 12),
            ("ex:following-sibling", "following-sibling", 20),
            ("ex:preceding", "preceding", 12),
            ("ex:preceding-sibling", "preceding-sibling", 20),
            ("ex:namespace", "namespace", 12),
        ];
        for (input, local_name, len) in cases {
            let mut iter = ExplicitWhitespace::from_str(input);
            assert_eq!(
                iter.next(),
                Some((
                    Token::PrefixedQName(PrefixedQName {
                        prefix: "ex",
                        local_name
                    }),
                    0..len
                )),
                "failed for input: {input}"
            );
            assert_eq!(iter.next(), None, "unexpected trailing token for: {input}");
        }
    }

    #[test]
    fn test_prefixed_qname_with_keyword_local_names() {
        let cases = [
            ("ex:and", "and", 6),
            ("ex:or", "or", 5),
            ("ex:div", "div", 6),
            ("ex:mod", "mod", 6),
            ("ex:for", "for", 6),
            ("ex:let", "let", 6),
            ("ex:some", "some", 7),
            ("ex:every", "every", 8),
            ("ex:is", "is", 5),
            ("ex:to", "to", 5),
            ("ex:union", "union", 8),
            ("ex:intersect", "intersect", 12),
            ("ex:except", "except", 9),
            ("ex:instance", "instance", 11),
            ("ex:treat", "treat", 8),
            ("ex:cast", "cast", 7),
            ("ex:castable", "castable", 11),
            ("ex:as", "as", 5),
            ("ex:in", "in", 5),
            ("ex:return", "return", 9),
            ("ex:satisfies", "satisfies", 12),
            ("ex:then", "then", 7),
            ("ex:else", "else", 7),
        ];
        for (input, local_name, len) in cases {
            let mut iter = ExplicitWhitespace::from_str(input);
            assert_eq!(
                iter.next(),
                Some((
                    Token::PrefixedQName(PrefixedQName {
                        prefix: "ex",
                        local_name
                    }),
                    0..len
                )),
                "failed for input: {input}"
            );
            assert_eq!(iter.next(), None, "unexpected trailing token for: {input}");
        }
    }

    // Keywords should also work as prefixes in QNames
    #[test]
    fn test_prefixed_qname_with_keyword_prefix() {
        let mut iter = ExplicitWhitespace::from_str("child:foo");
        assert_eq!(
            iter.next(),
            Some((
                Token::PrefixedQName(PrefixedQName {
                    prefix: "child",
                    local_name: "foo"
                }),
                0..9
            ))
        );
        assert_eq!(iter.next(), None);
    }
}
