use std::rc::Rc;

use ibig::ibig;
use iri_string::types::IriReferenceStr;
use xot::Xot;

use crate::context::DocumentsRef;
use crate::context::DynamicContext;
use crate::context::StaticContext;
use crate::error::SpannedError;
use crate::function::Function;
use crate::interpreter::interpret::ContextInfo;
use crate::sequence;
use crate::stack;
use crate::{error, string};

use super::program::FunctionInfo;
use super::Interpreter;
use super::Program;

#[derive(Debug)]
pub struct Runnable<'a> {
    program: &'a Program,
    // TODO: this should be private, but is needed right now
    // to implement call_static without lifetime issues.
    // We could possibly obtain context from the interpreter directly,
    // but this leads to lifetime issues right now.
    pub(crate) dynamic_context: &'a DynamicContext<'a>,
}

impl<'a> Runnable<'a> {
    pub(crate) fn new(program: &'a Program, dynamic_context: &'a DynamicContext) -> Self {
        Self {
            program,
            dynamic_context,
        }
    }

    fn run_value(&self, xot: &'a mut Xot) -> error::SpannedResult<stack::Value> {
        let arguments = self.dynamic_context.arguments().unwrap();
        let mut interpreter = Interpreter::new(self, xot);

        let context_info = if let Some(context_item) = self.dynamic_context.context_item() {
            ContextInfo {
                item: context_item.clone().into(),
                position: ibig!(1).into(),
                size: ibig!(1).into(),
            }
        } else {
            ContextInfo {
                item: stack::Value::Absent,
                position: stack::Value::Absent,
                size: stack::Value::Absent,
            }
        };

        interpreter.start(context_info, arguments);
        interpreter.run(0)?;

        let state = interpreter.state();
        // the stack has to be 1 values and return the result of the expression
        // why 1 value if the context item is on the top of the stack? This is because
        // the outer main function will pop the context item; this code is there to
        // remove the function id from the stack but the main function has no function id
        assert_eq!(
            state.stack().len(),
            1,
            "stack must only have 1 value but found {:?}",
            state.stack()
        );
        let value = state.stack().last().unwrap().clone();
        match value {
            stack::Value::Absent => Err(SpannedError {
                error: error::Error::XPDY0002,
                span: Some(self.program.span().into()),
            }),
            _ => Ok(value),
        }
    }

    /// Run the program against a sequence item.
    pub fn many(&self, xot: &'a mut Xot) -> error::SpannedResult<sequence::Sequence> {
        Ok(self.run_value(xot)?.try_into()?)
    }

    /// Run the program, expect a single item as the result.
    pub fn one(&self, xot: &'a mut Xot) -> error::SpannedResult<sequence::Item> {
        let sequence = self.many(xot)?;
        sequence.one().map_err(|error| SpannedError {
            error,
            span: Some(self.program.span().into()),
        })
    }

    /// Run the program, expect an optional single item as the result.
    pub fn option(&self, xot: &'a mut Xot) -> error::SpannedResult<Option<sequence::Item>> {
        let sequence = self.many(xot)?;
        let items = sequence.iter();
        sequence::option(items).map_err(|error| SpannedError {
            error,
            span: Some(self.program.span().into()),
        })
    }

    pub(crate) fn program(&self) -> &'a Program {
        self.program
    }

    pub fn dynamic_context(&self) -> &'a DynamicContext<'_> {
        self.dynamic_context
    }

    pub fn documents(&self) -> DocumentsRef {
        self.dynamic_context.documents()
    }

    pub fn static_context(&self) -> &StaticContext {
        self.program.static_context()
    }

    pub fn default_collation_uri(&self) -> &IriReferenceStr {
        self.dynamic_context
            .static_context()
            .default_collation_uri()
    }

    pub fn default_collation(&self) -> error::Result<Rc<string::Collation>> {
        self.dynamic_context.static_context().default_collation()
    }

    pub fn implicit_timezone(&self) -> chrono::FixedOffset {
        self.dynamic_context.implicit_timezone()
    }

    pub fn function_info<'b>(&self, function: &'b Function) -> FunctionInfo<'a, 'b> {
        self.program.function_info(function)
    }
}
