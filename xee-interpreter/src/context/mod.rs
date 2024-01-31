/// The static context is used during compile time. It is then used to
/// construct a dynamic context, which is used during runtime.
mod dynamic_context;
mod static_context;

pub use dynamic_context::DynamicContext;
pub use static_context::StaticContext;
