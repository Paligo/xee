/// The static context is used during compile time. It is then used to
/// construct a dynamic context, which is used during runtime.
mod dynamic_context;
mod dynamic_context_builder;
mod static_context;
mod static_context_builder;

pub use dynamic_context::{DynamicContext, Variables};
pub use dynamic_context_builder::{DocumentsRef, DynamicContextBuilder, TypeTableRef};
pub use static_context::StaticContext;
pub use static_context_builder::StaticContextBuilder;
