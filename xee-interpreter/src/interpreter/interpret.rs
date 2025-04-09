use std::cmp::Ordering;
use std::rc::Rc;

use ibig::{ibig, IBig};

use xee_name::Name;
use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xot::xmlname::NameStrInfo;
use xot::Xot;

use crate::atomic::{self, AtomicCompare};
use crate::atomic::{
    op_add, op_div, op_idiv, op_mod, op_multiply, op_subtract, OpEq, OpGe, OpGt, OpLe, OpLt, OpNe,
};
use crate::context::DynamicContext;
use crate::function;
use crate::pattern::PredicateMatcher;
use crate::sequence;
use crate::span::SourceSpan;
use crate::stack;
use crate::xml;
use crate::{error, pattern};

use super::instruction::{read_i16, read_instruction, read_u16, read_u8, EncodedInstruction};
use super::runnable::Runnable;
use super::state::State;

pub struct Interpreter<'a> {
    runnable: &'a Runnable<'a>,
    pub(crate) state: State<'a>,
}

pub struct ContextInfo {
    pub item: stack::Value,
    pub position: stack::Value,
    pub size: stack::Value,
}

impl From<sequence::Item> for ContextInfo {
    fn from(item: sequence::Item) -> Self {
        ContextInfo {
            item: item.into(),
            position: ibig!(1).into(),
            size: ibig!(1).into(),
        }
    }
}

impl<'a> Interpreter<'a> {
    pub fn new(runnable: &'a Runnable<'a>, xot: &'a mut Xot) -> Self {
        Interpreter {
            runnable,
            state: State::new(xot),
        }
    }

    pub fn state(self) -> State<'a> {
        self.state
    }

    pub(crate) fn runnable(&self) -> &Runnable {
        self.runnable
    }

    pub fn start(&mut self, context_info: ContextInfo, arguments: Vec<sequence::Sequence>) {
        self.start_function(self.runnable.program().main_id(), context_info, arguments)
    }

    fn start_function(
        &mut self,
        function_id: function::InlineFunctionId,
        context_info: ContextInfo,
        arguments: Vec<sequence::Sequence>,
    ) {
        self.state.push_start_frame(function_id);

        self.push_context_info(context_info);
        // and any arguments
        for arg in arguments {
            self.state.push(arg);
        }
    }

    fn push_context_info(&mut self, context_info: ContextInfo) {
        self.state.push_value(context_info.item);
        self.state.push_value(context_info.position);
        self.state.push_value(context_info.size);
    }

    pub fn run(&mut self, start_base: usize) -> error::SpannedResult<()> {
        // annotate run with detailed error information
        self.run_actual(start_base).map_err(|e| self.err(e))
    }

    pub(crate) fn run_actual(&mut self, start_base: usize) -> error::Result<()> {
        // we can make this an infinite loop as all functions end
        // with the return instruction
        loop {
            let instruction = self.read_instruction();
            match instruction {
                EncodedInstruction::Add => {
                    self.arithmetic_with_offset(op_add)?;
                }
                EncodedInstruction::Sub => {
                    self.arithmetic_with_offset(op_subtract)?;
                }
                EncodedInstruction::Mul => {
                    self.arithmetic(op_multiply)?;
                }
                EncodedInstruction::Div => {
                    self.arithmetic(op_div)?;
                }
                EncodedInstruction::IntDiv => {
                    self.arithmetic(op_idiv)?;
                }
                EncodedInstruction::Mod => {
                    self.arithmetic(op_mod)?;
                }
                EncodedInstruction::Plus => {
                    self.unary_arithmetic(|a| a.plus())?;
                }
                EncodedInstruction::Minus => {
                    self.unary_arithmetic(|a| a.minus())?;
                }
                EncodedInstruction::Concat => {
                    let (a, b) = self.pop_atomic2_option()?;
                    let a = a.unwrap_or("".into());
                    let b = b.unwrap_or("".into());
                    let a = a.cast_to_string();
                    let b = b.cast_to_string();
                    let a = a.to_str().unwrap();
                    let b = b.to_str().unwrap();
                    let result = a.to_string() + b;
                    let item: sequence::Item = result.into();
                    self.state.push(item);
                }
                EncodedInstruction::Const => {
                    let index = self.read_u16();
                    self.state
                        .push(self.current_inline_function().constants[index as usize].clone());
                }
                EncodedInstruction::Closure => {
                    let function_id = self.read_u16();
                    let inline_function_id = function::InlineFunctionId(function_id as usize);
                    let closure_function =
                        self.runnable.program().inline_function(inline_function_id);

                    let mut closure_vars = Vec::with_capacity(closure_function.closure_names.len());
                    for _ in 0..closure_function.closure_names.len() {
                        closure_vars.push(self.state.pop_value());
                    }
                    let function: function::Function =
                        function::InlineFunctionData::new(inline_function_id, closure_vars).into();
                    let item: sequence::Item = function.into();
                    self.state.push(item);
                }
                EncodedInstruction::StaticClosure => {
                    let static_function_id = self.read_u16();
                    let static_function_id =
                        function::StaticFunctionId(static_function_id as usize);
                    let static_closure =
                        self.create_static_closure_from_stack(static_function_id)?;
                    let item: sequence::Item = static_closure.into();
                    self.state.push(item);
                }
                EncodedInstruction::Var => {
                    let index = self.read_u16();
                    self.state.push_var(index as usize);
                }
                EncodedInstruction::Set => {
                    let index = self.read_u16();
                    self.state.set_var(index as usize);
                }
                EncodedInstruction::ClosureVar => {
                    let index = self.read_u16();
                    self.state.push_closure_var(index as usize)?;
                }
                EncodedInstruction::Comma => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    let sequence = a.concat(b)?;
                    self.state.push(sequence);
                }
                EncodedInstruction::CurlyArray => {
                    let sequence = self.state.pop()?;
                    let array: function::Array = sequence.into();
                    self.state.push(array);
                }
                EncodedInstruction::SquareArray => {
                    let length = self.pop_atomic().unwrap();
                    let length = length.cast_to_integer_value::<i64>()?;
                    let mut popped: Vec<sequence::Sequence> = Vec::with_capacity(length as usize);
                    for _ in 0..length {
                        popped.push(self.state.pop()?);
                    }
                    let item: sequence::Item = function::Array::new(popped).into();
                    self.state.push(item);
                }
                EncodedInstruction::CurlyMap => {
                    let length = self.pop_atomic().unwrap();
                    let length = length.cast_to_integer_value::<i64>()?;
                    let mut popped: Vec<(atomic::Atomic, sequence::Sequence)> =
                        Vec::with_capacity(length as usize);
                    for _ in 0..length {
                        let value = self.state.pop()?;
                        let key = self.pop_atomic()?;
                        popped.push((key, value));
                    }
                    let item: sequence::Item = function::Map::new(popped)?.into();
                    self.state.push(item);
                }
                EncodedInstruction::Jump => {
                    let displacement = self.read_i16();
                    self.state.jump(displacement as i32);
                }
                EncodedInstruction::JumpIfTrue => {
                    let displacement = self.read_i16();
                    let a = self.pop_effective_boolean()?;
                    if a {
                        self.state.jump(displacement as i32);
                    }
                }
                EncodedInstruction::JumpIfFalse => {
                    let displacement = self.read_i16();
                    let a = self.pop_effective_boolean()?;
                    if !a {
                        self.state.jump(displacement as i32);
                    }
                }
                EncodedInstruction::Eq => {
                    self.value_compare(OpEq)?;
                }
                EncodedInstruction::Ne => self.value_compare(OpNe)?,
                EncodedInstruction::Lt => {
                    self.value_compare(OpLt)?;
                }
                EncodedInstruction::Le => {
                    self.value_compare(OpLe)?;
                }
                EncodedInstruction::Gt => {
                    self.value_compare(OpGt)?;
                }
                EncodedInstruction::Ge => {
                    self.value_compare(OpGe)?;
                }
                EncodedInstruction::GenEq => {
                    self.general_compare(OpEq)?;
                }
                EncodedInstruction::GenNe => {
                    self.general_compare(OpNe)?;
                }
                EncodedInstruction::GenLt => {
                    self.general_compare(OpLt)?;
                }
                EncodedInstruction::GenLe => {
                    self.general_compare(OpLe)?;
                }
                EncodedInstruction::GenGt => {
                    self.general_compare(OpGt)?;
                }
                EncodedInstruction::GenGe => {
                    self.general_compare(OpGe)?;
                }
                EncodedInstruction::Is => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    if a.is_empty() || b.is_empty() {
                        self.state.push(sequence::Sequence::default());
                        continue;
                    }
                    let result = a.is(&b)?;
                    self.state.push(result);
                }
                EncodedInstruction::Precedes => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    if a.is_empty() || b.is_empty() {
                        self.state.push(sequence::Sequence::default());
                        continue;
                    }
                    let result = a.precedes(
                        &b,
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(result);
                }
                EncodedInstruction::Follows => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    if a.is_empty() || b.is_empty() {
                        self.state.push(sequence::Sequence::default());
                        continue;
                    }
                    let result = a.follows(
                        &b,
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(result);
                }
                EncodedInstruction::Union => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    let combined = a.union(
                        b,
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(combined);
                }
                EncodedInstruction::Intersect => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    let combined = a.intersect(
                        b,
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(combined);
                }
                EncodedInstruction::Except => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    let combined = a.except(
                        b,
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(combined);
                }
                EncodedInstruction::Dup => {
                    let value = self.state.pop()?;
                    self.state.push(value.clone());
                    self.state.push(value);
                }
                EncodedInstruction::Pop => {
                    self.state.pop()?;
                }
                EncodedInstruction::Call => {
                    let arity = self.read_u8();
                    self.call(arity)?;
                }
                EncodedInstruction::Lookup => {
                    self.lookup()?;
                }
                EncodedInstruction::WildcardLookup => {
                    self.wildcard_lookup()?;
                }
                EncodedInstruction::Step => {
                    let step_id = self.read_u16();
                    let node: xot::Node = self.state.pop()?.try_into()?;
                    let step = &(self.current_inline_function().steps[step_id as usize]);
                    let value = xml::resolve_step(step, node, self.state.xot());
                    self.state.push(value);
                }
                EncodedInstruction::Deduplicate => {
                    let value = self.state.pop()?;
                    let value = value.deduplicate(
                        self.runnable
                            .documents()
                            .borrow()
                            .document_order_access(self.xot()),
                    )?;
                    self.state.push(value);
                }
                EncodedInstruction::Return => {
                    if self.state.inline_return(start_base) {
                        break;
                    }
                }
                EncodedInstruction::ReturnConvert => {
                    let sequence_type_id = self.read_u16();
                    let sequence = self.state.pop()?;
                    let sequence_type =
                        &(self.current_inline_function().sequence_types[sequence_type_id as usize]);

                    let sequence = sequence.sequence_type_matching_function_conversion(
                        sequence_type,
                        self.runnable.static_context(),
                        self.state.xot(),
                        &|function| self.runnable.function_info(function).signature(),
                    )?;
                    self.state.push(sequence);
                }
                EncodedInstruction::LetDone => {
                    let return_value = self.state.pop()?;
                    // pop the variable assignment
                    let _ = self.state.pop();
                    self.state.push(return_value);
                }
                EncodedInstruction::Cast => {
                    let type_id = self.read_u16();
                    let value = self.pop_atomic_option()?;
                    let cast_type = &(self.current_inline_function().cast_types[type_id as usize]);
                    if let Some(value) = value {
                        let cast_value = value
                            .cast_to_schema_type(cast_type.xs, self.runnable.static_context())?;
                        self.state.push(cast_value);
                    } else if cast_type.empty_sequence_allowed {
                        self.state.push(sequence::Sequence::default());
                    } else {
                        Err(error::Error::XPTY0004)?;
                    }
                }
                EncodedInstruction::Castable => {
                    let type_id = self.read_u16();
                    let value = self.pop_atomic_option()?;
                    let cast_type = &(self.current_inline_function().cast_types[type_id as usize]);
                    if let Some(value) = value {
                        let cast_value =
                            value.cast_to_schema_type(cast_type.xs, self.runnable.static_context());
                        self.state.push(cast_value.is_ok());
                    } else if cast_type.empty_sequence_allowed {
                        self.state.push(true)
                    } else {
                        self.state.push(false);
                    }
                }
                EncodedInstruction::InstanceOf => {
                    let sequence_type_id = self.read_u16();
                    let sequence = self.state.pop()?;
                    let sequence_type =
                        &(self.current_inline_function().sequence_types[sequence_type_id as usize]);
                    let matches = sequence.sequence_type_matching(
                        sequence_type,
                        self.state.xot(),
                        &|function| self.runnable.function_info(function).signature(),
                    );
                    if matches.is_ok() {
                        self.state.push(true);
                    } else {
                        self.state.push(false);
                    }
                }
                EncodedInstruction::Treat => {
                    let sequence_type_id = self.read_u16();
                    let sequence = self.state.top()?;
                    let sequence_type =
                        &(self.current_inline_function().sequence_types[sequence_type_id as usize]);
                    let matches = sequence.sequence_type_matching(
                        sequence_type,
                        self.state.xot(),
                        &|function| self.runnable.function_info(function).signature(),
                    );
                    if matches.is_err() {
                        Err(error::Error::XPDY0050)?;
                    }
                }
                EncodedInstruction::Range => {
                    let b = self.state.pop()?;
                    let a = self.state.pop()?;
                    let a = a.atomized_option(self.state.xot())?;
                    let b = b.atomized_option(self.state.xot())?;
                    let (a, b) = match (a, b) {
                        (None, None) | (None, _) | (_, None) => {
                            self.state.push(sequence::Sequence::default());
                            continue;
                        }
                        (Some(a), Some(b)) => (a, b),
                    };
                    // we want to ensure we have integers at this point;
                    // we don't want to be casting strings or anything
                    a.ensure_base_schema_type(Xs::Integer)?;
                    b.ensure_base_schema_type(Xs::Integer)?;

                    let a: IBig = a.try_into().unwrap();
                    let b: IBig = b.try_into().unwrap();

                    match a.cmp(&b) {
                        Ordering::Greater => self.state.push(sequence::Sequence::default()),
                        Ordering::Equal => self.state.push(a),
                        Ordering::Less => {
                            let sequence: sequence::Sequence =
                                sequence::Range::new(a, b + 1)?.into();
                            self.state.push(sequence)
                        }
                    }
                }

                EncodedInstruction::SequenceLen => {
                    let value = self.state.pop()?;
                    let l: IBig = value.len().into();
                    self.state.push(l);
                }
                EncodedInstruction::SequenceGet => {
                    let value = self.state.pop()?;
                    let index = self.pop_atomic()?;
                    let index = index.cast_to_integer_value::<i64>()? as usize;
                    // substract 1 as Xpath is 1-indexed
                    let item = value.get(index - 1).ok_or(error::Error::XPTY0004)?;
                    let sequence: sequence::Sequence = item.into();
                    self.state.push(sequence)
                }
                EncodedInstruction::BuildNew => {
                    self.state.build_new();
                }
                EncodedInstruction::BuildPush => {
                    self.state.build_push()?;
                }
                EncodedInstruction::BuildComplete => {
                    self.state.build_complete();
                }
                EncodedInstruction::IsNumeric => {
                    let is_numeric = self.pop_is_numeric()?;
                    self.state.push(is_numeric);
                }
                EncodedInstruction::XmlName => {
                    let local_name_value = self.pop_atomic()?;
                    let namespace_value = self.pop_atomic()?;
                    let namespace = namespace_value.to_str()?;
                    let local_name = local_name_value.to_string()?;
                    let name =
                        xee_name::Name::new(local_name, namespace.to_string(), String::new());
                    self.state.push(name);
                }
                EncodedInstruction::XmlDocument => {
                    let root_node = self.state.xot.new_document();
                    let item = sequence::Item::Node(root_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlElement => {
                    let name_id = self.pop_xot_name()?;
                    let element_node = self.state.xot.new_element(name_id);
                    let item = sequence::Item::Node(element_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlAttribute => {
                    let value = self.pop_atomic()?;
                    let name_id = self.pop_xot_name()?;
                    let attribute_node = self
                        .state
                        .xot
                        .new_attribute_node(name_id, value.string_value());
                    let item = sequence::Item::Node(attribute_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlNamespace => {
                    let uri = self.pop_atomic()?;
                    let namespace_id = self.state.xot.add_namespace(&uri.string_value());
                    let prefix = self.pop_atomic()?;
                    let prefix_id = self.state.xot.add_prefix(&prefix.string_value());
                    let namespace_node = self.state.xot.new_namespace_node(prefix_id, namespace_id);
                    let item = sequence::Item::Node(namespace_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlText => {
                    let text_atomic = self.pop_atomic()?;
                    let text = text_atomic.into_canonical();
                    let text_node = self.state.xot.new_text(&text);
                    let item = sequence::Item::Node(text_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlComment => {
                    let text_atomic = self.pop_atomic()?;
                    let text = text_atomic.into_canonical();
                    let comment_node = self.state.xot.new_comment(&text);
                    let item = sequence::Item::Node(comment_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlProcessingInstruction => {
                    let text_atomic = self.pop_atomic()?;
                    let text = text_atomic.into_canonical();
                    let text = if !text.is_empty() {
                        Some(text.as_str())
                    } else {
                        None
                    };
                    let target_atomic = self.pop_atomic()?;
                    let target = target_atomic.into_canonical();
                    let target_id = self.state.xot.add_name(&target);
                    let pi_node = self.state.xot.new_processing_instruction(target_id, text);
                    let item = sequence::Item::Node(pi_node);
                    self.state.push(item);
                }
                EncodedInstruction::XmlAppend => {
                    let child_value = self.state.pop()?;
                    let parent_node = self.pop_node()?;
                    self.xml_append(parent_node, child_value)?;
                    // now we can push back the parent node
                    let item = sequence::Item::Node(parent_node);
                    self.state.push(item);
                }
                EncodedInstruction::CopyShallow => {
                    let value = &self.state.pop()?;
                    if value.is_empty() {
                        self.state.push(sequence::Sequence::default());
                        continue;
                    }
                    if value.len() > 1 {
                        Err(error::Error::XTTE3180)?;
                    }
                    let item = value.iter().next().unwrap();
                    let copy = match &item {
                        sequence::Item::Atomic(_) | sequence::Item::Function(_) => item.clone(),
                        sequence::Item::Node(node) => {
                            let copied_node = self.shallow_copy_node(*node);
                            sequence::Item::Node(copied_node)
                        }
                    };
                    self.state.push(copy);
                }
                EncodedInstruction::CopyDeep => {
                    let value = &self.state.pop()?;
                    if value.is_empty() {
                        self.state.push(sequence::Sequence::default());
                        continue;
                    }
                    let mut new_sequence = Vec::with_capacity(value.len());
                    for item in value.iter() {
                        let copy = match &item {
                            sequence::Item::Atomic(_) | sequence::Item::Function(_) => item.clone(),
                            sequence::Item::Node(node) => {
                                let copied_node = self.state.xot.clone_node(*node);
                                sequence::Item::Node(copied_node)
                            }
                        };
                        new_sequence.push(copy);
                    }
                    self.state.push(new_sequence);
                }
                EncodedInstruction::ApplyTemplates => {
                    let value = self.state.pop()?;
                    let mode_id = self.read_u16();
                    let mode = pattern::ModeId::new(mode_id as usize);
                    let value = self.apply_templates_sequence(mode, value)?;
                    self.state.push(value);
                }
                EncodedInstruction::PrintTop => {
                    let top = self.state.top()?;
                    println!("{:#?}", top);
                }
                EncodedInstruction::PrintStack => {
                    println!("{:#?}", self.state.stack());
                }
            }
        }
        Ok(())
    }

    pub(crate) fn create_static_closure_from_stack(
        &mut self,
        static_function_id: function::StaticFunctionId,
    ) -> error::Result<function::Function> {
        Self::create_static_closure(self.runnable.dynamic_context(), static_function_id, || {
            Some(self.state.pop_value())
        })
    }

    pub(crate) fn create_static_closure_from_context(
        &mut self,
        static_function_id: function::StaticFunctionId,
        arg: Option<xot::Node>,
    ) -> error::Result<function::Function> {
        Self::create_static_closure(self.runnable.dynamic_context(), static_function_id, || {
            arg.map(|n| {
                let value: stack::Value = n.into();
                value
            })
        })
    }

    pub(crate) fn create_static_closure<F>(
        context: &DynamicContext,
        static_function_id: function::StaticFunctionId,
        mut get: F,
    ) -> error::Result<function::Function>
    where
        F: FnMut() -> Option<stack::Value>,
    {
        let static_function = &context.static_context().function_by_id(static_function_id);
        // get any context value from the stack if needed
        let closure_vars = if static_function.needs_context() {
            let value = get();
            if let Some(value) = value {
                vec![value]
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        Ok(function::StaticFunctionData::new(static_function_id, closure_vars).into())
    }

    pub(crate) fn current_inline_function(&self) -> &function::InlineFunction {
        self.runnable
            .program()
            .inline_function(self.state.frame().function())
    }

    pub(crate) fn function_name(&self, function: &function::Function) -> Option<Name> {
        self.runnable.function_info(function).name()
    }

    pub(crate) fn function_arity(&self, function: &function::Function) -> usize {
        self.runnable.function_info(function).arity()
    }

    fn call(&mut self, arity: u8) -> error::Result<()> {
        let function = self.state.callable(arity as usize)?;
        self.call_function(&function, arity)
    }

    pub(crate) fn call_function_with_arguments(
        &mut self,
        function: &function::Function,
        arguments: &[sequence::Sequence],
    ) -> error::Result<sequence::Sequence> {
        // put function onto the stack
        let item: sequence::Item = function.clone().into();
        self.state.push(item);
        // then arguments
        let arity = arguments.len() as u8;
        for arg in arguments.iter() {
            self.state.push(arg.clone());
        }
        self.call_function(function, arity)?;
        if matches!(function, function::Function::Inline(_)) {
            // run interpreter until we return to the base
            // we started in
            self.run_actual(self.state.frame().base())?;
        }
        self.state.pop()
    }

    fn call_function(&mut self, function: &function::Function, arity: u8) -> error::Result<()> {
        match function {
            function::Function::Static(data) => {
                self.call_static(data.id, arity, &data.closure_vars)
            }
            function::Function::Inline(data) => self.call_inline(data.id, arity),
            function::Function::Array(array) => self.call_array(array, arity as usize),
            function::Function::Map(map) => self.call_map(map, arity as usize),
        }
    }

    pub(crate) fn arguments(&self, arity: u8) -> &[stack::Value] {
        self.state.arguments(arity as usize)
    }

    fn call_static(
        &mut self,
        static_function_id: function::StaticFunctionId,
        arity: u8,
        closure_vars: &[stack::Value],
    ) -> error::Result<()> {
        let static_function = self.runnable.program().static_function(static_function_id);
        if arity as usize != static_function.arity() {
            return Err(error::Error::XPTY0004);
        }
        let parameter_types = static_function.signature().parameter_types();
        let arguments = self.coerce_arguments(parameter_types, arity)?;
        let result =
            static_function.invoke(self.runnable.dynamic_context, self, arguments, closure_vars)?;
        // pop the last item off
        let _ = self.state.pop();
        self.state.push(result);
        Ok(())
    }

    fn call_inline(
        &mut self,
        function_id: function::InlineFunctionId,
        arity: u8,
    ) -> error::Result<()> {
        // look up the function in order to access the parameters information
        let function = self.runnable.program().inline_function(function_id);
        let parameter_types = &function.signature.parameter_types();
        if arity as usize != parameter_types.len() {
            return Err(error::Error::XPTY0004);
        }

        let arguments = self.coerce_arguments(parameter_types, arity)?;

        // now we have a list of arguments that we want to push back onto the stack
        // (they are already reversed)
        for arg in arguments {
            self.state.push(arg);
        }

        self.state.push_frame(function_id, arity as usize)
    }

    fn coerce_arguments(
        &mut self,
        parameter_types: &[Option<ast::SequenceType>],
        arity: u8,
    ) -> error::Result<Vec<sequence::Sequence>> {
        // TODO: fast path if no sequence type declarations exist for
        // parameters could cache this inside of signature so that it's really
        // fast to detect.

        // we could also have a secondary fast path where if the types are all
        // exactly the same, we don't do a clone.

        // get all the stack values out in order
        let stack_values = self.state.arguments(arity as usize);
        let mut arguments = Vec::with_capacity(arity as usize);
        let static_context = self.runnable.static_context();
        let xot = self.state.xot();
        for (parameter_type, stack_value) in parameter_types.iter().zip(stack_values) {
            let sequence: sequence::Sequence = stack_value.try_into()?;
            if let Some(type_) = parameter_type {
                // matching also takes care of function conversion rules
                let sequence = sequence.sequence_type_matching_function_conversion(
                    type_,
                    static_context,
                    xot,
                    &|function| self.runnable.function_info(function).signature(),
                )?;
                arguments.push(sequence);
            } else {
                // no need to do any checking or conversion
                arguments.push(sequence);
            }
        }
        self.state.truncate_arguments(arity as usize);
        Ok(arguments)
    }

    fn call_array(&mut self, array: &function::Array, arity: usize) -> error::Result<()> {
        if arity != 1 {
            return Err(error::Error::XPTY0004);
        }
        // the argument
        let position = self.pop_atomic()?;
        let sequence = Self::array_get(array, position)?;
        // pop the array off the stack
        self.state.pop()?;
        // now push the result
        self.state.push(sequence);
        Ok(())
    }

    fn array_get(
        array: &function::Array,
        position: atomic::Atomic,
    ) -> error::Result<sequence::Sequence> {
        let position = position
            .cast_to_integer_value::<i64>()
            .map_err(|_| error::Error::XPTY0004)?;
        let position = position as usize;
        if position == 0 {
            return Err(error::Error::FOAY0001);
        }
        let position = position - 1;
        let sequence = array.index(position);
        sequence.cloned().ok_or(error::Error::FOAY0001)
    }

    fn call_map(&mut self, map: &function::Map, arity: usize) -> error::Result<()> {
        if arity != 1 {
            return Err(error::Error::XPTY0004);
        }
        let key = self.pop_atomic()?;
        let value = map.get(&key);
        // pop the map off the stack
        self.state.pop()?;
        if let Some(value) = value {
            self.state.push(value.clone());
        } else {
            self.state.push(sequence::Sequence::default());
        }
        Ok(())
    }

    fn lookup(&mut self) -> error::Result<()> {
        let key_specifier = self.state.pop()?;
        let value = self.state.pop()?;
        let function: function::Function = value.try_into()?;
        let value = self.lookup_value(&function, key_specifier)?;
        let sequence: sequence::Sequence = value.into();
        self.state.push(sequence);
        Ok(())
    }

    fn lookup_value(
        &self,
        function: &function::Function,
        key_specifier: sequence::Sequence,
    ) -> error::Result<Vec<sequence::Item>> {
        match function {
            function::Function::Map(map) => self.lookup_map(map, key_specifier),
            function::Function::Array(array) => self.lookup_array(array, key_specifier),
            _ => Err(error::Error::XPTY0004),
        }
    }

    fn lookup_map(
        &self,
        map: &function::Map,
        key_specifier: sequence::Sequence,
    ) -> error::Result<Vec<sequence::Item>> {
        self.lookup_helper(key_specifier, map, |map, atomic| {
            Ok(map.get(&atomic).cloned().unwrap_or_default())
        })
    }

    fn lookup_array(
        &self,
        array: &function::Array,
        key_specifier: sequence::Sequence,
    ) -> error::Result<Vec<sequence::Item>> {
        self.lookup_helper(key_specifier, array, |array, atomic| match atomic {
            atomic::Atomic::Integer(..) => Self::array_get(array, atomic),
            _ => Err(error::Error::XPTY0004),
        })
    }

    fn lookup_helper<T>(
        &self,
        key_specifier: sequence::Sequence,
        data: T,
        get_key: impl Fn(&T, atomic::Atomic) -> error::Result<sequence::Sequence>,
    ) -> error::Result<Vec<sequence::Item>> {
        let keys = key_specifier
            .atomized(self.state.xot())
            .collect::<error::Result<Vec<_>>>()?;
        let mut result = Vec::new();
        for key in keys {
            for item in get_key(&data, key)?.iter() {
                result.push(item.clone());
            }
        }
        Ok(result)
    }

    fn wildcard_lookup(&mut self) -> error::Result<()> {
        let value = self.state.pop()?;
        let function: function::Function = value.try_into()?;
        let value = match function {
            function::Function::Map(map) => {
                let mut result = Vec::new();
                for key in map.keys() {
                    for value in self.lookup_map(&map, key.clone().into())? {
                        result.push(value)
                    }
                }
                result
            }
            function::Function::Array(array) => {
                let mut result = Vec::new();
                for i in 1..(array.len() + 1) {
                    let i: IBig = i.into();
                    for value in self.lookup_array(&array, i.into())? {
                        result.push(value)
                    }
                }
                result
            }
            _ => return Err(error::Error::XPTY0004),
        };
        let sequence: sequence::Sequence = value.into();
        self.state.push(sequence);
        Ok(())
    }

    fn value_compare<O>(&mut self, op: O) -> error::Result<()>
    where
        O: AtomicCompare,
    {
        let b = self.state.pop()?;
        let a = self.state.pop()?;
        // https://www.w3.org/TR/xpath-31/#id-value-comparisons
        // If an operand is the empty sequence, the result is the empty sequence
        if a.is_empty() || b.is_empty() {
            self.state.push(sequence::Sequence::default());
            return Ok(());
        }
        let v = a.value_compare(
            &b,
            op,
            self.runnable.default_collation()?.as_ref(),
            self.runnable.implicit_timezone(),
            self.state.xot(),
        )?;
        self.state.push(v);
        Ok(())
    }

    fn general_compare<O>(&mut self, op: O) -> error::Result<()>
    where
        O: AtomicCompare,
    {
        let b = self.state.pop()?;
        let a = self.state.pop()?;
        let value =
            a.general_comparison(&b, op, self.runnable.dynamic_context(), self.state.xot())?;
        self.state.push(value);
        Ok(())
    }

    fn arithmetic<F>(&mut self, op: F) -> error::Result<()>
    where
        F: Fn(atomic::Atomic, atomic::Atomic) -> error::Result<atomic::Atomic>,
    {
        self.arithmetic_with_offset(|a, b, _| op(a, b))
    }

    fn arithmetic_with_offset<F>(&mut self, op: F) -> error::Result<()>
    where
        F: Fn(atomic::Atomic, atomic::Atomic, chrono::FixedOffset) -> error::Result<atomic::Atomic>,
    {
        let b = self.state.pop()?;
        let a = self.state.pop()?;
        // https://www.w3.org/TR/xpath-31/#id-arithmetic
        // 2. If an operand is the empty sequence, the result is the empty sequence
        if a.is_empty() || b.is_empty() {
            self.state.push(sequence::Sequence::default());
            return Ok(());
        }
        let a = a.atomized_one(self.state.xot())?;
        let b = b.atomized_one(self.state.xot())?;
        let result = op(a, b, self.runnable.implicit_timezone())?;
        self.state.push(result);
        Ok(())
    }

    fn unary_arithmetic<F>(&mut self, op: F) -> error::Result<()>
    where
        F: Fn(atomic::Atomic) -> error::Result<atomic::Atomic>,
    {
        let a = self.state.pop()?;
        if a.is_empty() {
            self.state.push(sequence::Sequence::default());
            return Ok(());
        }
        let a = a.atomized_one(self.state.xot())?;
        let value = op(a)?;
        self.state.push(value);
        Ok(())
    }

    fn pop_is_numeric(&mut self) -> error::Result<bool> {
        let value = self.state.pop()?;
        let a = value.atomized_option(self.state.xot())?;
        if let Some(a) = a {
            Ok(a.is_numeric())
        } else {
            Ok(false)
        }
    }

    fn pop_atomic(&mut self) -> error::Result<atomic::Atomic> {
        let value = self.state.pop()?;
        value.atomized_one(self.state.xot())
    }

    fn pop_atomic_option(&mut self) -> error::Result<Option<atomic::Atomic>> {
        let value = self.state.pop()?;
        value.atomized_option(self.state.xot())
    }

    fn pop_xot_name(&mut self) -> error::Result<xot::NameId> {
        let value = self.pop_atomic()?;
        let name: xee_name::Name = value.try_into()?;
        let namespace = name.namespace();
        let ns = self.state.xot.add_namespace(namespace);
        Ok(self.state.xot.add_name_ns(name.local_name(), ns))
    }

    fn pop_node(&mut self) -> error::Result<xot::Node> {
        let value = self.state.pop()?;
        let node = value.one()?.to_node()?;
        Ok(node)
    }

    fn pop_atomic2(&mut self) -> error::Result<(atomic::Atomic, atomic::Atomic)> {
        let b = self.pop_atomic()?;
        let a = self.pop_atomic()?;
        Ok((a, b))
    }

    fn pop_atomic2_option(
        &mut self,
    ) -> error::Result<(Option<atomic::Atomic>, Option<atomic::Atomic>)> {
        let b = self.pop_atomic_option()?;
        let a = self.pop_atomic_option()?;
        Ok((a, b))
    }

    fn pop_effective_boolean(&mut self) -> error::Result<bool> {
        let a = self.state.pop()?;
        a.effective_boolean_value()
    }

    pub(crate) fn regex(&self, pattern: &str, flags: &str) -> error::Result<Rc<regexml::Regex>> {
        self.state.regex(pattern, flags)
    }

    pub(crate) fn xot(&self) -> &Xot {
        self.state.xot()
    }

    pub(crate) fn xot_mut(&mut self) -> &mut Xot {
        self.state.xot_mut()
    }

    fn xml_append(
        &mut self,
        parent_node: xot::Node,
        value: sequence::Sequence,
    ) -> error::Result<()> {
        let mut string_values = Vec::new();
        for item in value.iter() {
            match item {
                sequence::Item::Node(node) => {
                    // if there were any string values before this node, add them
                    // to the node, separated by a space character
                    if !string_values.is_empty() {
                        self.xml_append_string_values(parent_node, &string_values);
                        string_values.clear();
                    }
                    match self.state.xot.value(node) {
                        xot::Value::Document => {
                            // TODO: Handle adding all the children instead
                            return Err(error::Error::Unsupported);
                        }
                        xot::Value::Text(text) => {
                            // zero length text nodes are skipped
                            // Can this even exist, or does Xot not have
                            // them anyway?
                            if text.get().is_empty() {
                                continue;
                            }
                        }
                        _ => {}
                    }

                    // if we have a parent we're already in another document,
                    // in which case we want to make a clone first
                    let node = if self.state.xot.parent(node).is_some() {
                        self.state.xot.clone_node(node)
                    } else {
                        node
                    };
                    // TODO: error out if namespace or attribute node
                    // is added once a normal child already exists
                    self.state.xot.any_append(parent_node, node).unwrap();
                }
                sequence::Item::Atomic(atomic) => string_values.push(atomic.string_value()),
                sequence::Item::Function(_) => return Err(error::Error::XTDE0450),
            }
        }
        // if there are any string values left in the end
        if !string_values.is_empty() {
            self.xml_append_string_values(parent_node, &string_values);
        }
        Ok(())
    }

    fn xml_append_string_values(&mut self, parent_node: xot::Node, string_values: &[String]) {
        let text = string_values.join(" ");
        let text_node = self.state.xot.new_text(&text);
        self.state.xot.append(parent_node, text_node).unwrap();
    }

    fn shallow_copy_node(&mut self, node: xot::Node) -> xot::Node {
        let xot = &mut self.state.xot;
        let value = xot.value(node);
        match value {
            // root and element are shallow copies
            xot::Value::Document => xot.new_document(),
            // TODO: work on copying prefixes
            xot::Value::Element(element) => xot.new_element(element.name()),
            // we can clone (deep-copy) these nodes as it's the same
            // operation as shallow copy
            _ => xot.clone_node(node),
        }
    }

    fn apply_templates_sequence(
        &mut self,
        mode: pattern::ModeId,
        sequence: sequence::Sequence,
    ) -> error::Result<sequence::Sequence> {
        let mut r: Vec<sequence::Item> = Vec::new();
        let size: IBig = sequence.len().into();

        for (i, item) in sequence.iter().enumerate() {
            let sequence = self.apply_templates_item(mode, item, i, size.clone())?;
            if let Some(sequence) = sequence {
                for item in sequence.iter() {
                    r.push(item.clone());
                }
            }
        }
        Ok(r.into())
    }

    fn apply_templates_item(
        &mut self,
        mode: pattern::ModeId,
        item: sequence::Item,
        position: usize,
        size: IBig,
    ) -> error::Result<Option<sequence::Sequence>> {
        let function_id = self.lookup_pattern(mode, &item);

        if let Some(function_id) = function_id {
            let position: IBig = (position + 1).into();
            let arguments: Vec<sequence::Sequence> = vec![
                item.into(),
                atomic::Atomic::from(position).into(),
                atomic::Atomic::from(size.clone()).into(),
            ];
            let function = function::InlineFunctionData::new(function_id, Vec::new()).into();
            self.call_function_with_arguments(&function, &arguments)
                .map(Some)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn lookup_pattern(
        &mut self,
        mode: pattern::ModeId,
        item: &sequence::Item,
    ) -> Option<function::InlineFunctionId> {
        self.runnable
            .program()
            .declarations
            .mode_lookup
            .lookup(mode, |pattern| self.matches(pattern, item))
            .copied()
    }

    // The interpreter can return an error for any byte code, in any level of
    // nesting in the function. When this happens the interpreter stops with
    // the error code. We here wrap it in a SpannedError using the current
    // span.
    pub(crate) fn err(&self, value_error: error::Error) -> error::SpannedError {
        error::SpannedError {
            error: value_error,
            span: Some(self.current_span()),
        }
    }

    // During the compilation process, spans became associated with each
    // compiled bytecode instruction. Here we take the current function and the
    // instruction in it to determine the span of the code that failed.
    fn current_span(&self) -> SourceSpan {
        let frame = self.state.frame();
        let function = self.runnable.program().inline_function(frame.function());
        // we substract 1 to end up in the current instruction - this
        // because the ip is already on the next instruction
        function.spans[frame.ip - 1]
    }

    fn read_instruction(&mut self) -> EncodedInstruction {
        let frame = self.state.frame_mut();
        let function = self.runnable.program().inline_function(frame.function());
        let chunk = &function.chunk;
        read_instruction(chunk, &mut frame.ip)
    }

    fn read_u16(&mut self) -> u16 {
        let frame = &mut self.state.frame_mut();
        let function = self.runnable.program().inline_function(frame.function());
        let chunk = &function.chunk;
        read_u16(chunk, &mut frame.ip)
    }

    fn read_i16(&mut self) -> i16 {
        let frame = &mut self.state.frame_mut();
        let function = self.runnable.program().inline_function(frame.function());
        let chunk = &function.chunk;
        read_i16(chunk, &mut frame.ip)
    }

    fn read_u8(&mut self) -> u8 {
        let frame = &mut self.state.frame_mut();
        let function = self.runnable.program().inline_function(frame.function());
        let chunk = &function.chunk;
        read_u8(chunk, &mut frame.ip)
    }
}
