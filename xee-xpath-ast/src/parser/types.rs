use chumsky::inspector::SimpleState;
use chumsky::{extra::Full, prelude::*};

use std::borrow::Cow;

use crate::error::ParserError;
use crate::Namespaces;
pub(crate) struct State<'a> {
    pub(crate) namespaces: Cow<'a, Namespaces>,
}

type Extra<'a> = Full<ParserError, SimpleState<State<'a>>, ()>;

pub(crate) type BoxedParser<'a, I, T> = Boxed<'a, 'a, I, T, Extra<'a>>;
