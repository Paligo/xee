/// XML integration.
mod annotation;
mod annotation2;
mod base;
mod document;
mod kind_test;
mod step;

pub(crate) use annotation2::DocumentOrderAccess;
pub(crate) use base::BaseUriResolver;
pub use document::{Document, DocumentHandle, Documents, DocumentsError};
pub(crate) use kind_test::kind_test;
pub(crate) use step::resolve_step;
pub use step::Step;
