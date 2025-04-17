mod annotation;
/// XPath can be extended with both static functions as well as user defined
/// functions.
mod array;
mod function_core;
mod inline_function;
mod map;
mod signature;
mod static_function;

pub use annotation::Annotations;
pub use array::Array;
pub use function_core::Function;
pub use function_core::{
    InlineFunctionData, InlineFunctionId, StaticFunctionData, StaticFunctionId,
};
pub use inline_function::{CastType, InlineFunction, Name};
pub use map::Map;
pub use signature::Signature;

// we allow StaticFunctionType as it's used in the xpath_fn macro
pub use static_function::FunctionRule;
#[allow(unused_imports)]
pub(crate) use static_function::StaticFunctionType;
pub(crate) use static_function::{FunctionKind, StaticFunctionDescription};
pub(crate) use static_function::{StaticFunction, StaticFunctions};
