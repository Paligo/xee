mod ast_ir;
mod priority;
mod run;

pub use ast_ir::{parse, parse_with_base_dir};
pub use run::evaluate;
