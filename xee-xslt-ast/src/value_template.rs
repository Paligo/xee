use xee_xpath_ast::{ast as xpath_ast, ParserError, XPathParserContext};

use crate::ast_core as ast;
use crate::ast_core::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ValueTemplateItem<'a> {
    String { text: &'a str, span: Span },
    Curly { c: char },
    Value { xpath: xpath_ast::XPath, span: Span },
}

impl<'a> From<ValueTemplateItem<'a>> for ast::ValueTemplateItem {
    fn from(item: ValueTemplateItem<'a>) -> Self {
        match item {
            ValueTemplateItem::String { text, span } => Self::String {
                text: text.to_string(),
                span,
            },
            ValueTemplateItem::Curly { c } => Self::Curly { c },
            ValueTemplateItem::Value { xpath, span } => {
                Self::Value { xpath, span }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Error {
    UnescapedCurly { c: char, span: Span },
    IllegalSlice,
    XPath(ParserError),
}

impl From<ParserError> for Error {
    fn from(e: ParserError) -> Self {
        Self::XPath(e)
    }
}

pub(crate) struct ValueTemplateTokenizer<'a> {
    s: &'a str,
    char_indices: std::iter::Peekable<std::str::CharIndices<'a>>,
    span: Span,
    mode: Mode,
    start: usize,
    parser_context: &'a XPathParserContext,
    done: bool,
}

enum Mode {
    String,
    Value,
    StartCurly,
    EndCurly,
}

impl<'a> ValueTemplateTokenizer<'a> {
    pub(crate) fn new(s: &'a str, span: Span, parser_context: &'a XPathParserContext) -> Self {
        Self {
            s,
            char_indices: s.char_indices().peekable(),
            span,
            mode: Mode::String,
            start: 0,
            parser_context,
            done: false,
        }
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span {
            start: self.span.start + start,
            end: self.span.start + end,
        }
    }

    fn string_item(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<Result<ValueTemplateItem<'a>, Error>> {
        if let Some(text) = self.s.get(start..end) {
            if text.is_empty() {
                return self.next();
            }
            Some(Ok(ValueTemplateItem::String {
                text,
                span: self.span(start, end),
            }))
        } else {
            Some(Err(Error::IllegalSlice))
        }
    }

    fn start_curly_item(&self) -> Result<ValueTemplateItem<'a>, Error> {
        Ok(ValueTemplateItem::Curly { c: '{' })
    }

    fn end_curly_item(&self) -> Result<ValueTemplateItem<'a>, Error> {
        Ok(ValueTemplateItem::Curly { c: '}' })
    }
}

impl<'a> Iterator for ValueTemplateTokenizer<'a> {
    type Item = Result<ValueTemplateItem<'a>, Error>;

    fn next(&mut self) -> Option<Result<ValueTemplateItem<'a>, Error>> {
        if self.done {
            return None;
        }
        let start = self.start;
        let end;
        match self.mode {
            Mode::String => loop {
                if let Some((i, c)) = self.char_indices.next() {
                    match c {
                        '{' => {
                            self.mode = Mode::StartCurly;
                            end = i;
                            self.start = end + 1;
                            return self.string_item(start, end);
                        }
                        '}' => {
                            self.mode = Mode::EndCurly;
                            end = i;
                            self.start = end + 1;
                            return self.string_item(start, end);
                        }
                        _ => {
                            continue;
                        }
                    }
                } else {
                    end = self.s.len();
                    self.done = true;
                    return self.string_item(start, end);
                }
            },
            Mode::Value => {
                // if we have an immediate } following, we skip this value
                // entirely as if it doesn't exist
                // TODO: parse whitespace, xpath comments too
                if let Some((_peek_i, peek_c)) = self.char_indices.peek() {
                    if *peek_c == '}' {
                        self.start = start + 1;
                        self.char_indices.next();
                        self.mode = Mode::String;
                        return self.next();
                    }
                }
                let xpath = self
                    .parser_context
                    .parse_value_template_xpath(&self.s[self.start..]);
                match xpath {
                    Ok(xpath) => {
                        let span = xpath.0.span;
                        // slurp up from the char iterator, including }
                        for _ in 0..span.end + 1 {
                            self.char_indices.next();
                        }
                        // construct span of value
                        let new_span = self.span(start, self.start + span.end);
                        self.start = self.start + span.end + 1;
                        self.mode = Mode::String;

                        // we successfully parsed an xpath expression, with
                        // an additional closing }
                        Some(Ok(ValueTemplateItem::Value {
                            xpath,
                            span: new_span,
                        }))
                    }
                    Err(e) => {
                        self.done = true;
                        Some(Err(Error::XPath(e.adjust(self.start))))
                    }
                }
            }

            Mode::StartCurly => {
                if let Some((_peek_i, peek_c)) = self.char_indices.peek() {
                    if *peek_c == '{' {
                        // TODO: refactor to use next without unpacking
                        if let Some((i, _c)) = self.char_indices.next() {
                            self.mode = Mode::String;
                            end = i + 1;
                            self.start = end;
                            return Some(self.start_curly_item());
                        } else {
                            unreachable!();
                        }
                    }
                }
                self.mode = Mode::Value;
                self.next()
            }
            Mode::EndCurly => {
                if let Some((_peek_i, peek_c)) = self.char_indices.peek() {
                    if *peek_c == '}' {
                        if let Some((i, _c)) = self.char_indices.next() {
                            self.mode = Mode::String;
                            end = i + 1;
                            self.start = end;
                            return Some(self.end_curly_item());
                        } else {
                            unreachable!();
                        }
                    }
                }
                self.done = true;
                Some(Err(Error::UnescapedCurly {
                    c: '}',
                    span: Span {
                        start: self.span.start + self.start,
                        end: self.span.start + self.start + 1,
                    },
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_ron_snapshot;

    fn parse_with_span<'a>(
        s: &'a str,
        span: Span,
        parser_context: &'a XPathParserContext,
    ) -> Result<Vec<ValueTemplateItem<'a>>, Error> {
        let tokenizer = ValueTemplateTokenizer::new(s, span, parser_context);
        tokenizer.collect()
    }

    fn parse<'a>(
        s: &'a str,
        parser_context: &'a XPathParserContext,
    ) -> Result<Vec<ValueTemplateItem<'a>>, Error> {
        let span = Span {
            start: 0,
            end: s.len(),
        };
        parse_with_span(s, span, parser_context)
    }

    #[test]
    fn test_string_without_curly() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello world", &parser_context));
    }

    #[test]
    fn test_string_start_curly_escaped() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello{{world", &parser_context));
    }

    #[test]
    fn test_string_end_curly_escaped() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello}}world", &parser_context));
    }

    #[test]
    fn test_string_with_value() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello {world}!", &parser_context));
    }

    #[test]
    fn test_string_with_value_in_span() {
        let parser_context = XPathParserContext::default();

        let s = "hello {world}!";
        let span = Span {
            start: 10,
            end: s.len() + 10,
        };
        assert_ron_snapshot!(parse_with_span(s, span, &parser_context));
    }

    #[test]
    fn test_string_with_empty_value() {
        let parser_context = XPathParserContext::default();

        assert_ron_snapshot!(parse("hello {}!", &parser_context));
    }

    #[test]
    fn test_string_with_multiple_values() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello {a} and {b}!", &parser_context));
    }

    #[test]
    fn test_string_with_multiple_adjacent_values() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello {a}{b}!", &parser_context));
    }

    #[test]
    fn test_string_unescaped_unclosed_start_curly() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello{world", &parser_context));
    }

    #[test]
    fn test_string_unescaped_unclosed_start_curly_with_span() {
        let parser_context = XPathParserContext::default();
        let s = "hello{world";
        let span = Span {
            start: 10,
            end: 10 + s.len(),
        };
        assert_ron_snapshot!(parse_with_span(s, span, &parser_context));
    }

    #[test]
    fn test_string_unescaped_end_curly() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello}world", &parser_context));
    }

    #[test]
    fn test_broken_xpath() {
        let parser_context = XPathParserContext::default();
        assert_ron_snapshot!(parse("hello {a +}!", &parser_context));
    }
}
