#[cfg(feature = "ariadne")]
mod ariadne;
mod span;

pub use span::{SourceId, SourceOrigin, SourceSpan};
