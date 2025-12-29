use std::rc::Rc;

use ahash::AHashMap;
use ibig::ibig;
use iri_string::types::IriReferenceStr;
use xot::Xot;

use crate::context::DocumentsRef;
use crate::context::DynamicContext;
use crate::context::StaticContext;
use crate::error::SpannedError;
use crate::function::{Function, InlineFunctionData};
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
        let arguments = self.resolve_global_param_arguments(xot)?;
        let mut interpreter = Interpreter::new(self, xot, self.dynamic_context.type_table());

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

    fn resolve_global_param_arguments(
        &self,
        xot: &'a mut Xot,
    ) -> error::SpannedResult<Vec<sequence::Sequence>> {
        if self.program.declarations.global_params.is_empty() {
            return Ok(self.dynamic_context.arguments()?);
        }
        let globals = &self.program.declarations.global_params;
        let mut explicit: AHashMap<xot::xmlname::OwnedName, sequence::Sequence> =
            AHashMap::new();
        for (name, value) in self.dynamic_context.variables() {
            explicit.insert(name.clone(), value.clone());
        }
        let mut values: AHashMap<xot::xmlname::OwnedName, sequence::Sequence> = AHashMap::new();
        for global in globals {
            if global.overrideable {
                if let Some(value) = explicit.get(&global.name) {
                    values.insert(global.name.clone(), value.clone());
                }
            }
        }
        let iterations = globals.len().max(1);

        for _ in 0..iterations {
            for global in globals {
                let value = if global.overrideable {
                    if let Some(value) = explicit.get(&global.name) {
                        value.clone()
                    } else if let Some(default_fn) = global.default {
                        let args = globals
                            .iter()
                            .map(|param| values.get(&param.name).cloned().unwrap_or_default())
                            .collect::<Vec<_>>();
                        let function = InlineFunctionData::new(default_fn, Vec::new()).into();
                        let mut interpreter = Interpreter::new(self, xot, self.dynamic_context.type_table());
                        interpreter
                            .call_function_with_arguments(&function, &args)
                            .map_err(|error| error::SpannedError {
                                error,
                                span: Some(self.program.span().into()),
                            })?
                    } else if global.required {
                        return Err(error::SpannedError {
                            error: error::Error::XTDE0050,
                            span: Some(self.program.span().into()),
                        });
                    } else {
                        sequence::Sequence::default()
                    }
                } else if let Some(default_fn) = global.default {
                    let args = globals
                        .iter()
                        .map(|param| values.get(&param.name).cloned().unwrap_or_default())
                        .collect::<Vec<_>>();
                    let function = InlineFunctionData::new(default_fn, Vec::new()).into();
                    let mut interpreter = Interpreter::new(self, xot, self.dynamic_context.type_table());
                    interpreter
                        .call_function_with_arguments(&function, &args)
                        .map_err(|error| error::SpannedError {
                            error,
                            span: Some(self.program.span().into()),
                        })?
                } else if global.required {
                    return Err(error::SpannedError {
                        error: error::Error::XTDE0050,
                        span: Some(self.program.span().into()),
                    });
                } else {
                    sequence::Sequence::default()
                };

                values.insert(global.name.clone(), value);
            }
        }

        let mut resolved = Vec::with_capacity(globals.len());
        for global in globals {
            resolved.push(values.get(&global.name).cloned().unwrap_or_default());
        }
        Ok(resolved)
    }

    /// Run the program against a sequence item.
    pub fn many(&self, xot: &'a mut Xot) -> error::SpannedResult<sequence::Sequence> {
        Ok(self.run_value(xot)?.try_into()?)
    }

    pub fn call_named_template(
        &self,
        xot: &'a mut Xot,
        name: &xot::xmlname::OwnedName,
        params: Option<&AHashMap<xot::xmlname::OwnedName, sequence::Sequence>>,
    ) -> error::SpannedResult<sequence::Sequence> {
        let function_id = self
            .program
            .declarations
            .named_templates
            .get(name)
            .copied()
            .ok_or(SpannedError {
                error: error::Error::Unsupported(String::from("Named template not found")),
                span: Some(self.program.span().into()),
            })?;

        let arguments = self.resolve_global_param_arguments(xot)?;
        let mut interpreter = Interpreter::new(self, xot, self.dynamic_context.type_table());
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
        interpreter
            .call_named_template(function_id, params)
            .map_err(|error| SpannedError {
                error,
                span: Some(self.program.span().into()),
            })
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
