//! This module makes [`SourceSpan`] available as an implementation of the [`chumsky::span::Span`] trait,
//! so it can be used by the [`chumsky`] crate.  
use std::ops::Range;

use crate::span::{SourceId, SourceSpan};

impl chumsky::span::Span for SourceSpan {
    type Context = SourceId;
    type Offset = usize;

    fn new(context: Self::Context, range: Range<Self::Offset>) -> Self {
        Self::new(context, range)
    }

    fn context(&self) -> Self::Context {
        self.source_id
    }

    fn start(&self) -> Self::Offset {
        self.start
    }

    fn end(&self) -> Self::Offset {
        self.end
    }
}
