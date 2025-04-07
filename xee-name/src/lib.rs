#![warn(missing_docs)]

//! Manage namespaces in Xee.

mod namespaces;
mod variable_names;

pub use namespaces::{NamespaceLookup, Namespaces, DEFAULT_NAMESPACES, FN_NAMESPACE, XS_NAMESPACE};
pub use variable_names::VariableNames;
pub use xot::xmlname::OwnedName as Name;
