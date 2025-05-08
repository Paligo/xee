use std::borrow::Cow;

use crate::ParserSource;

pub(crate) struct WhitespaceSplitter<'a> {
    source: &'a ParserSource<'a>,
    char_indices: std::str::CharIndices<'a>,
}

impl<'a> WhitespaceSplitter<'a> {
    pub(crate) fn new(source: &'a ParserSource<'a>) -> Self {
        Self {
            source,
            char_indices: source.source_text().char_indices(),
        }
    }
}

impl<'a> Iterator for WhitespaceSplitter<'a> {
    type Item = ParserSource<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut start;
        // we skip any whitespace characters, then take the first
        // non-whitespace sequence we get
        loop {
            if let Some((i, c)) = self.char_indices.next() {
                start = i;
                if !c.is_whitespace() {
                    break;
                }
            } else {
                return None;
            }
        }
        // now we take as many non-whitespace characters we find
        let mut end;
        loop {
            if let Some((i, c)) = self.char_indices.next() {
                end = i;
                if c.is_whitespace() {
                    break;
                }
            } else {
                end = self.source.source_text().len();
                break;
            }
        }

        let part_source = ParserSource {
            source_text: Cow::Borrowed(&self.source.source_text()[start..end]),
            source_id: self.source.source_id,
            adjust: self.source.adjust + start,
        };

        Some(part_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_whitespace_with_spans_simple() {
        let s = "hello world";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 2);
        assert_eq!(splitted[0].source_text(), "hello");
        assert_eq!(splitted[0].adjust, 0);
        assert_eq!(splitted[1].source_text(), "world");
        assert_eq!(splitted[1].adjust, 6);
    }

    #[test]
    fn test_split_whitespace_with_spans_long_whitespace() {
        let s = "hello   world";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 2);
        assert_eq!(splitted[0].source_text(), "hello");
        assert_eq!(splitted[0].adjust, 0);
        assert_eq!(splitted[1].source_text(), "world");
        assert_eq!(splitted[1].adjust, 8);
    }

    #[test]
    fn test_split_whitespace_multiple() {
        let s = "alpha beta gamma";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 3);
        assert_eq!(splitted[0].source_text(), "alpha");
        assert_eq!(splitted[0].adjust, 0);
        assert_eq!(splitted[1].source_text(), "beta");
        assert_eq!(splitted[1].adjust, 6);
        assert_eq!(splitted[2].source_text(), "gamma");
        assert_eq!(splitted[2].adjust, 11);
    }

    #[test]
    fn test_no_whitespace() {
        let s = "alpha";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 1);
        assert_eq!(splitted[0].source_text(), "alpha");
        assert_eq!(splitted[0].adjust, 0);
    }

    #[test]
    fn test_leading_whitespace() {
        let s = "  alpha";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 1);
        assert_eq!(splitted[0].source_text(), "alpha");
        assert_eq!(splitted[0].adjust, 2);
    }

    #[test]
    fn test_trailing_whitespace() {
        let s = "alpha  ";
        let source = ParserSource::dynamic(s);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 1);
        assert_eq!(splitted[0].source_text(), "alpha");
        assert_eq!(splitted[0].adjust, 0);
    }

    #[test]
    fn test_adjust() {
        let s = "alpha beta gamma";
        let source = ParserSource::dynamic_adjusted(s, 5);
        let splitted: Vec<_> = WhitespaceSplitter::new(&source).collect();
        assert_eq!(splitted.len(), 3);
        assert_eq!(splitted[0].source_text(), "alpha");
        assert_eq!(splitted[0].adjust, 5);
        assert_eq!(splitted[1].source_text(), "beta");
        assert_eq!(splitted[1].adjust, 11);
        assert_eq!(splitted[2].source_text(), "gamma");
        assert_eq!(splitted[2].adjust, 16);
    }
}
