use std::fmt::Formatter;
use std::rc::Rc;

use ibig::IBig;
use xee_name::Name;
use xot::Xot;

use crate::atomic::Atomic;
use crate::context::DynamicContext;
use crate::context::StaticContext;
use crate::error::SpannedError;
use crate::function;
use crate::occurrence::Occurrence;
use crate::sequence;
use crate::stack;
use crate::xml;
use crate::{error, string};

use super::Interpreter;
use super::Program;

#[derive(Debug, Clone)]
pub struct Runnable<'a> {
    program: &'a Program,
    map_signature: function::Signature,
    array_signature: function::Signature,
    // TODO: this should be private, but is needed right now
    // to implement call_static without lifetime issues.
    // We could possibly obtain context from the interpreter directly,
    // but this leads to lifetime issues right now.
    pub(crate) dynamic_context: &'a DynamicContext<'a>,
}

struct RunValue {
    output: Xot,
    value: stack::Value,
}

pub struct SequenceOutput {
    pub output: Xot,
    pub sequence: sequence::Sequence,
}

impl std::fmt::Display for SequenceOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for item in self.sequence.items() {
            write!(
                f,
                "{}",
                self.output
                    .to_string(item.unwrap().to_node().unwrap().xot_node())
                    .unwrap()
            )?;
        }
        Ok(())
    }
}

impl<'a> Runnable<'a> {
    pub(crate) fn new(program: &'a Program, dynamic_context: &'a DynamicContext<'a>) -> Self {
        Self {
            program,
            map_signature: function::Signature::map_signature(),
            array_signature: function::Signature::array_signature(),
            dynamic_context,
        }
    }

    fn run_value(&self, context_item: Option<&sequence::Item>) -> error::SpannedResult<RunValue> {
        let mut interpreter = Interpreter::new(self);
        // TODO: the arguments aren't supplied to the function that are expected.
        // This should result in an error, preferrably the variable that is missing
        // underlined in the xpath expression. But that requires some more work to
        // accomplish, so for now we panic.
        let arguments = self.dynamic_context.arguments().unwrap();
        interpreter.start(context_item.map(|item| item.clone().into()), arguments);
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
        let output = state.output();
        match value {
            stack::Value::Absent => Err(SpannedError {
                error: error::Error::XPDY0002,
                span: self.program.span().into(),
            }),
            _ => Ok(RunValue { output, value }),
        }
    }

    /// Run the program against a sequence item.
    pub fn many(&self, item: Option<&sequence::Item>) -> error::SpannedResult<sequence::Sequence> {
        let value = self.run_value(item)?;
        Ok(value.value.into())
    }

    /// Run the program against a sequence item.
    ///
    /// Also deliver output Xot.
    pub fn many_output(
        &self,
        item: Option<&sequence::Item>,
    ) -> error::SpannedResult<SequenceOutput> {
        let value = self.run_value(item)?;
        Ok(SequenceOutput {
            output: value.output,
            sequence: value.value.into(),
        })
    }

    /// Run the program against a xot Node.
    pub fn many_xot_node(&self, node: xot::Node) -> error::SpannedResult<sequence::Sequence> {
        let node = xml::Node::Xot(node);
        let item = sequence::Item::Node(node);
        self.many(Some(&item))
    }

    /// Run the program, expect a single item as the result.
    pub fn one(&self, item: Option<&sequence::Item>) -> error::SpannedResult<sequence::Item> {
        let sequence = self.many(item)?;
        sequence.items().one().map_err(|error| SpannedError {
            error,
            span: self.program.span().into(),
        })
    }

    /// Run the program, expect an optional single item as the result.
    pub fn option(
        &self,
        item: Option<&sequence::Item>,
    ) -> error::SpannedResult<Option<sequence::Item>> {
        let sequence = self.many(item)?;
        sequence.items().option().map_err(|error| SpannedError {
            error,
            span: self.program.span().into(),
        })
    }

    pub fn apply_templates_sequence(
        &self,
        interpreter: &mut Interpreter,
        sequence: sequence::Sequence,
    ) -> error::SpannedResult<stack::Value> {
        let mut r: Vec<sequence::Item> = Vec::new();
        let size: IBig = sequence.len().into();
        for (i, item) in sequence.items().enumerate() {
            let item = item.unwrap(); // TODO
            let function_id = self
                .program
                .declarations
                .pattern_lookup
                .lookup(&item, self.dynamic_context.xot());
            if let Some(function_id) = function_id {
                let position: IBig = (i + 1).into();
                let arguments: Vec<sequence::Sequence> = vec![
                    item.into(),
                    Atomic::from(position).into(),
                    Atomic::from(size.clone()).into(),
                ];
                let function = Rc::new(function::Function::Inline {
                    inline_function_id: *function_id,
                    closure_vars: Vec::new(),
                });
                let value = interpreter
                    .call_function_with_arguments(function, &arguments)
                    .map_err(|e| interpreter.err(e))?;
                for item in value.items() {
                    r.push(item.unwrap());
                }
            }
        }
        Ok(r.into())
    }

    pub(crate) fn program(&self) -> &'a Program {
        self.program
    }

    pub fn dynamic_context(&self) -> &'a DynamicContext {
        self.dynamic_context
    }

    pub(crate) fn annotations(&self) -> &xml::Annotations {
        &self.dynamic_context.documents.annotations
    }

    pub fn static_context(&self) -> &StaticContext {
        self.dynamic_context.static_context
    }

    pub fn xot(&self) -> &xot::Xot {
        self.dynamic_context.xot()
    }

    pub fn default_collation_uri(&self) -> &str {
        self.dynamic_context.static_context.default_collation_uri()
    }

    pub fn default_collation(&self) -> error::Result<Rc<string::Collation>> {
        self.dynamic_context.static_context.default_collation()
    }

    pub fn implicit_timezone(&self) -> chrono::FixedOffset {
        self.dynamic_context.implicit_timezone()
    }

    pub(crate) fn inline_function(
        &self,
        function_id: function::InlineFunctionId,
    ) -> &'a function::InlineFunction {
        &self.program.functions[function_id.0]
    }

    pub(crate) fn static_function(
        &self,
        function_id: function::StaticFunctionId,
    ) -> &'a function::StaticFunction {
        self.dynamic_context
            .static_context
            .functions
            .get_by_index(function_id)
    }

    pub(crate) fn function_info<'function>(
        &'a self,
        function: &'function function::Function,
    ) -> FunctionInfo<'a, 'function> {
        FunctionInfo::new(function, self)
    }

    pub fn signature(&'a self, function: &function::Function) -> &'a function::Signature {
        self.function_info(function).signature()
    }
}

pub(crate) struct FunctionInfo<'runnable, 'function> {
    function: &'function function::Function,
    runnable: &'runnable Runnable<'runnable>,
}

impl<'runnable, 'function> FunctionInfo<'runnable, 'function> {
    pub(crate) fn new(
        function: &'function function::Function,
        runnable: &'runnable Runnable<'runnable>,
    ) -> FunctionInfo<'runnable, 'function> {
        FunctionInfo { function, runnable }
    }

    pub(crate) fn arity(&self) -> usize {
        match self.function {
            function::Function::Inline {
                inline_function_id, ..
            } => self.runnable.inline_function(*inline_function_id).arity(),
            function::Function::Static {
                static_function_id, ..
            } => self.runnable.static_function(*static_function_id).arity(),
            function::Function::Array(_) => 1,
            function::Function::Map(_) => 1,
        }
    }

    pub(crate) fn name(&self) -> Option<Name> {
        match self.function {
            function::Function::Static {
                static_function_id, ..
            } => {
                let static_function = self.runnable.static_function(*static_function_id);
                Some(static_function.name().clone())
            }
            _ => None,
        }
    }

    pub(crate) fn signature(&self) -> &'runnable function::Signature {
        match &self.function {
            function::Function::Static {
                static_function_id, ..
            } => {
                let static_function = self.runnable.static_function(*static_function_id);
                static_function.signature()
            }
            function::Function::Inline {
                inline_function_id, ..
            } => {
                let inline_function = self.runnable.inline_function(*inline_function_id);
                inline_function.signature()
            }
            function::Function::Map(_map) => &self.runnable.map_signature,
            function::Function::Array(_array) => &self.runnable.array_signature,
        }
    }
}
