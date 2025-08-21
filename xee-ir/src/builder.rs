use xee_xpath_ast::ast;

use xee_interpreter::interpreter::instruction::{
    encode_instruction, instruction_size, Instruction,
};
use xee_interpreter::{context, function, interpreter, sequence, span, xml};

use crate::ir;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ForwardJumpRef(usize);

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BackwardJumpRef(usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum JumpCondition {
    Always,
    True,
    False,
}

pub struct FunctionBuilder<'a> {
    program: &'a mut interpreter::Program,
    compiled: Vec<u8>,
    spans: Vec<span::SourceSpan>,
    constants: Vec<sequence::Sequence>,
    steps: Vec<xml::Step>,
    cast_types: Vec<function::CastType>,
    sequence_types: Vec<ast::SequenceType>,
    closure_names: Vec<ir::Name>,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(program: &'a mut interpreter::Program) -> Self {
        FunctionBuilder {
            program,
            compiled: Vec::new(),
            spans: Vec::new(),
            constants: Vec::new(),
            steps: Vec::new(),
            cast_types: Vec::new(),
            sequence_types: Vec::new(),
            closure_names: Vec::new(),
        }
    }

    pub(crate) fn static_context(&self) -> &context::StaticContext {
        self.program.static_context()
    }

    pub(crate) fn emit(&mut self, instruction: Instruction, span: span::SourceSpan) {
        for _ in 0..instruction_size(&instruction) {
            self.spans.push(span);
        }
        encode_instruction(instruction, &mut self.compiled);
    }

    pub(crate) fn emit_constant(&mut self, constant: sequence::Sequence, span: span::SourceSpan) {
        let constant_id = self.constants.len();
        self.constants.push(constant);
        if constant_id > (u16::MAX as usize) {
            panic!("too many constants");
        }
        self.emit(Instruction::Const(constant_id as u16), span);
    }

    pub(crate) fn add_closure_name(&mut self, name: &ir::Name) -> usize {
        let found = self.closure_names.iter().position(|n| n == name);
        if let Some(index) = found {
            return index;
        }
        let index = self.closure_names.len();
        self.closure_names.push(name.clone());
        if index > (u16::MAX as usize) {
            panic!("too many closure names");
        }
        index
    }

    pub(crate) fn add_step(&mut self, step: xml::Step) -> usize {
        let step_id = self.steps.len();
        self.steps.push(step);
        if step_id > (u16::MAX as usize) {
            panic!("too many steps");
        }
        step_id
    }

    pub(crate) fn add_cast_type(&mut self, cast_type: function::CastType) -> usize {
        let cast_type_id = self.cast_types.len();
        self.cast_types.push(cast_type);
        if cast_type_id > (u16::MAX as usize) {
            panic!("too many cast types");
        }
        cast_type_id
    }

    pub(crate) fn add_sequence_type(&mut self, sequence_type: ast::SequenceType) -> usize {
        let sequence_type_id = self.sequence_types.len();
        self.sequence_types.push(sequence_type);
        if sequence_type_id > (u16::MAX as usize) {
            panic!("too many sequence types");
        }
        sequence_type_id
    }

    pub(crate) fn loop_start(&self) -> BackwardJumpRef {
        BackwardJumpRef(self.compiled.len())
    }

    pub(crate) fn emit_jump_backward(
        &mut self,
        jump_ref: BackwardJumpRef,
        condition: JumpCondition,
        span: span::SourceSpan,
    ) {
        let current = self.compiled.len() + 3;
        let offset = current - jump_ref.0;
        if jump_ref.0 > current {
            panic!("cannot jump forward");
        }
        if offset > (u16::MAX as usize) {
            panic!("jump too far");
        }

        match condition {
            JumpCondition::True => self.emit(Instruction::JumpIfTrue(-(offset as i16)), span),
            JumpCondition::False => self.emit(Instruction::JumpIfFalse(-(offset as i16)), span),
            JumpCondition::Always => self.emit(Instruction::Jump(-(offset as i16)), span),
        }
    }

    pub(crate) fn emit_jump_forward(
        &mut self,
        condition: JumpCondition,
        span: span::SourceSpan,
    ) -> ForwardJumpRef {
        let index = self.compiled.len();
        match condition {
            JumpCondition::True => self.emit(Instruction::JumpIfTrue(0), span),
            JumpCondition::False => self.emit(Instruction::JumpIfFalse(0), span),
            JumpCondition::Always => self.emit(Instruction::Jump(0), span),
        }
        ForwardJumpRef(index)
    }

    pub(crate) fn patch_jump(&mut self, jump_ref: ForwardJumpRef) {
        let current = self.compiled.len();
        if jump_ref.0 > current {
            panic!("can only patch forward jumps");
        }
        let offset = current - jump_ref.0 - 3; // 3 for size of the jump
        if offset > (u16::MAX as usize) {
            panic!("jump too far");
        }
        let offset_bytes = offset.to_le_bytes();
        self.compiled[jump_ref.0 + 1] = offset_bytes[0];
        self.compiled[jump_ref.0 + 2] = offset_bytes[1];
    }

    pub(crate) fn finish(
        mut self,
        name: String,
        function_definition: &ir::FunctionDefinition,
        span: span::SourceSpan,
    ) -> function::InlineFunction {
        if let Some(return_type) = &function_definition.return_type {
            let sequence_type_id = self.add_sequence_type(return_type.clone());
            if sequence_type_id > (u16::MAX as usize) {
                panic!("too many sequence types");
            }
            self.emit(Instruction::ReturnConvert(sequence_type_id as u16), span);
        }
        self.emit(Instruction::Return, span);
        function::InlineFunction {
            name,
            signature: function_definition.signature(),
            chunk: self.compiled,
            spans: self.spans,
            closure_names: self.closure_names,
            constants: self.constants,
            steps: self.steps,
            cast_types: self.cast_types,
            sequence_types: self.sequence_types,
        }
    }

    pub(crate) fn builder(&mut self) -> FunctionBuilder<'_> {
        FunctionBuilder::new(self.program)
    }

    pub(crate) fn add_function(
        &mut self,
        function: function::InlineFunction,
    ) -> function::InlineFunctionId {
        self.program.add_function(function)
    }
}
