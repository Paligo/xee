/// The core of the interpreter: bytecodes and a way to run them. Bytecodes
/// are contained in functions, which together are composed into a program.
pub mod instruction;
mod interpret;
mod program;
mod runnable;
mod state;

pub use interpret::Interpreter;
pub use program::Program;
pub use runnable::{Runnable, SequenceOutput, Variables};
