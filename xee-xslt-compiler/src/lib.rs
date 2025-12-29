mod ast_ir;
mod default_declarations;
mod priority;
mod run;

pub use ast_ir::{parse, parse_with_base};
pub use run::evaluate;
