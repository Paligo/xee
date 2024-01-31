mod ast_ir;
mod compile;
mod query;
mod run;
mod span;

pub use xee_xpath_ast::ast::Name;
pub use xee_xpath_ast::{Namespaces, VariableNames};

pub use xee_interpreter::interpreter::Runnable;
pub use xee_interpreter::{
    atomic, context, error, interpreter::Variables, occurrence, sequence, string, xml,
};

pub use crate::ast_ir::IrConverter;
pub use crate::compile::{compile, parse};
pub use crate::query::{
    Convert, ManyQuery, OneQuery, OptionQuery, Queries, Query, Recurse, Session,
};
pub use crate::run::{
    evaluate, evaluate_root, evaluate_without_focus, evaluate_without_focus_with_variables,
};
