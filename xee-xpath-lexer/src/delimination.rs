use std::iter::Peekable;

use logos::{Logos, Span, SpannedIter};

use crate::explicit_whitespace::ExplicitWhitespace;
use crate::symbol_type::SymbolType;
use crate::Token;

pub struct DeliminationIterator<'a> {
    base: Peekable<ExplicitWhitespace<'a>>,
}

impl<'a> DeliminationIterator<'a> {
    pub(crate) fn new(base: ExplicitWhitespace<'a>) -> Self {
        Self {
            base: base.peekable(),
        }
    }

    pub(crate) fn from_spanned(spanned_iter: SpannedIter<'a, Token<'a>>) -> Self {
        let base = ExplicitWhitespace::new(spanned_iter);
        Self::new(base)
    }

    pub(crate) fn from_str(input: &'a str) -> Self {
        let spanned_lexer = Token::lexer(input).spanned();
        Self::from_spanned(spanned_lexer)
    }
}

impl<'a> Iterator for DeliminationIterator<'a> {
    type Item = (Token<'a>, Span);

    fn next(&mut self) -> Option<Self::Item> {
        let (token, span) = self.base.next()?;

        // IntegerLiteral won't be found with a dot behind it, as it would
        // become a decimal literal
        if matches!(&token, Token::DecimalLiteral(_) | Token::DoubleLiteral(_)) {
            let next = self.base.peek();
            if let Some((Token::Dot, _)) = next {
                return Some((Token::Error, span));
            }
        }

        // we don't have to handle the case of dot followed by a numeric literal
        // that starts with a dot itself, as this is lexed into Token::DotDot and
        // the parser won't accept DotDot in a stranger position. I hope.

        match token.symbol_type() {
            SymbolType::NonDelimiting => {
                // if we are not followed by either a delimiting symbol or a symbol separator, or are
                // at the end we are in error
                let next = self.base.peek();
                if let Some((next_token, _)) = next {
                    match next_token.symbol_type() {
                        SymbolType::Delimiting
                        | SymbolType::Whitespace
                        | SymbolType::CommentStart
                        | SymbolType::Error => {
                            // a non-delimiting symbol can be followed by these without error
                            Some((token, span))
                        }
                        SymbolType::NonDelimiting => {
                            // a non-delimiting symbol may not be followed by another one,
                            // so this is an error
                            Some((Token::Error, span))
                        }
                    }
                } else {
                    // if we are at the end we're delimited
                    Some((token, span))
                }
            }
            SymbolType::Delimiting | SymbolType::Error => Some((token, span)),
            SymbolType::Whitespace => {
                // we suppress whitespace
                self.next()
            }
            SymbolType::CommentStart => {
                // we skip comments
                self.next()
            }
        }
    }
}

pub fn lexer(input: &str) -> DeliminationIterator<'_> {
    DeliminationIterator::from_str(input)
}

#[cfg(test)]
mod tests {
    use crate::lexer::PrefixedQName;

    use super::*;

    use ibig::ibig;
    use rust_decimal_macros::dec;

    #[test]
    fn test_delimination() {
        let mut d = DeliminationIterator::from_str("a  b");

        assert_eq!(d.next(), Some((Token::NCName("a"), 0..1)));
        assert_eq!(d.next(), Some((Token::NCName("b"), 3..4)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_delimination_comment() {
        // because comments are turned into whitespace and whitespace is
        // collapsed, this shouldn't be a problem
        let mut d = DeliminationIterator::from_str("a (: foo :) b");

        assert_eq!(d.next(), Some((Token::NCName("a"), 0..1)));
        assert_eq!(d.next(), Some((Token::NCName("b"), 12..13)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_delimination_two_non_delimiting_without_separator() {
        let mut d = DeliminationIterator::from_str("1comment");

        assert_eq!(d.next(), Some((Token::Error, 0..1)));
        assert_eq!(d.next(), Some((Token::Comment, 1..8)));
    }

    #[test]
    fn test_delimination_non_delimiting_followed_by_delimiting() {
        let mut d = DeliminationIterator::from_str("1=");

        assert_eq!(d.next(), Some((Token::IntegerLiteral(ibig!(1)), 0..1)));
        assert_eq!(d.next(), Some((Token::Equal, 1..2)));
        assert_eq!(d.next(), None);
    }

    // if T is an NCName and U is "-" or ".", then the
    // lexer will absorb the "-" and "." at the end of
    // the ncname. This is a valid NCName and should be
    // accepted.

    #[test]
    fn test_ncname_followed_by_dot() {
        let mut d = DeliminationIterator::from_str("foo.");

        assert_eq!(d.next(), Some((Token::NCName("foo."), 0..4)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_prefixed_name_followed_by_dot() {
        let mut d = DeliminationIterator::from_str("foo:bar.");

        assert_eq!(
            d.next(),
            Some((
                Token::PrefixedQName(PrefixedQName {
                    prefix: "foo",
                    local_name: "bar."
                }),
                0..8
            ))
        );
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_ncname_followed_by_dash() {
        let mut d = DeliminationIterator::from_str("foo-");

        assert_eq!(d.next(), Some((Token::NCName("foo-"), 0..4)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_prefixed_name_followed_by_dash() {
        let mut d = DeliminationIterator::from_str("foo:bar-");

        assert_eq!(
            d.next(),
            Some((
                Token::PrefixedQName(PrefixedQName {
                    prefix: "foo",
                    local_name: "bar-"
                }),
                0..8
            ))
        );
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_numeric_followed_by_dot() {
        let mut d = DeliminationIterator::from_str("1..");

        assert_eq!(d.next(), Some((Token::Error, 0..2)));
        assert_eq!(d.next(), Some((Token::Dot, 2..3)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_dot_followed_by_a_number() {
        let mut d = DeliminationIterator::from_str("..1");

        assert_eq!(d.next(), Some((Token::DotDot, 0..2)));
        assert_eq!(d.next(), Some((Token::IntegerLiteral(ibig!(1)), 2..3)));
        assert_eq!(d.next(), None);
    }

    #[test]
    fn test_two_decimal_numbers() {
        let mut d = DeliminationIterator::from_str(".1.1");

        assert_eq!(d.next(), Some((Token::Error, 0..2)));
        assert_eq!(d.next(), Some((Token::DecimalLiteral(dec!(0.1)), 2..4)));
        assert_eq!(d.next(), None);
    }
}
