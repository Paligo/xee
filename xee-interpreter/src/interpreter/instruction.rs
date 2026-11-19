use num::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RaisedError {
    XTDE0700,
    XTTE0570,
}

impl RaisedError {
    pub(crate) fn to_u16(self) -> u16 {
        match self {
            RaisedError::XTDE0700 => 0,
            RaisedError::XTTE0570 => 1,
        }
    }

    pub(crate) fn from_u16(value: u16) -> Self {
        match value {
            0 => RaisedError::XTDE0700,
            1 => RaisedError::XTTE0570,
            _ => panic!("unknown raised error id: {value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    // binary operators
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    // unary operators
    Plus,
    Minus,
    //
    Concat,
    Absent,
    Const(u16),
    Closure(u16),
    NamedTemplate(u16),
    StaticClosure(u16),
    Var(u16),
    GlobalVar(u16),
    VarIsAbsent(u16),
    Set(u16),
    ClosureVar(u16),
    Comma,
    CurlyArray,
    SquareArray,
    CurlyMap,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    GenEq,
    GenNe,
    GenLt,
    GenLe,
    GenGt,
    GenGe,
    Is,
    Precedes,
    Follows,
    Union,
    Intersect,
    Except,
    Jump(i16),
    JumpIfTrue(i16),
    JumpIfFalse(i16),
    Call(u8),
    Lookup,
    WildcardLookup,
    Step(u16),
    Deduplicate,
    Return,
    ReturnConvert(u16),
    ConvertSequence(u16, RaisedError),
    Dup,
    Pop,
    LetDone,
    Cast(u16),
    Castable(u16),
    InstanceOf(u16),
    Treat(u16),
    Range,
    SequenceLen,
    SequenceGet,
    BuildNew,
    BuildPush,
    BuildComplete,
    IsNumeric,
    XmlName,
    XmlDocument,
    XmlElement,
    XmlAttribute,
    XmlNamespace,
    XmlText,
    XmlComment,
    XmlProcessingInstruction,
    XmlAppend,
    CopyShallow,
    CopyDeep,
    CallTemplate,
    ContinueTemplate,
    ApplyTemplates(u16, bool),
    ApplyTemplatesCurrent(u16, bool),
    RaiseError(RaisedError),
    PrintTop,
    PrintStack,
}

#[derive(Debug, ToPrimitive, FromPrimitive, Clone, PartialEq, Eq, Hash)]
pub(crate) enum EncodedInstruction {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Plus,
    Minus,
    Concat,
    Absent,
    Const,
    Closure,
    NamedTemplate,
    StaticClosure,
    Var,
    GlobalVar,
    VarIsAbsent,
    Set,
    ClosureVar,
    Comma,
    CurlyArray,
    SquareArray,
    CurlyMap,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    GenEq,
    GenNe,
    GenLt,
    GenLe,
    GenGt,
    GenGe,
    Is,
    Precedes,
    Follows,
    Union,
    Intersect,
    Except,
    Jump,
    JumpIfTrue,
    JumpIfFalse,
    Call,
    Lookup,
    WildcardLookup,
    Step,
    Deduplicate,
    Return,
    ReturnConvert,
    ConvertSequence,
    Dup,
    Pop,
    LetDone,
    Cast,
    Castable,
    InstanceOf,
    Treat,
    Range,
    SequenceLen,
    SequenceGet,
    BuildNew,
    BuildPush,
    BuildComplete,
    IsNumeric,
    XmlName,
    XmlDocument,
    XmlElement,
    XmlAttribute,
    XmlNamespace,
    XmlText,
    XmlComment,
    XmlProcessingInstruction,
    XmlAppend,
    CopyShallow,
    CopyDeep,
    CallTemplate,
    ContinueTemplate,
    ApplyTemplates,
    ApplyTemplatesCurrent,
    RaiseError,
    PrintTop,
    PrintStack,
}

// decode a single instruction from the slice
pub(crate) fn decode_instruction(bytes: &[u8]) -> (Instruction, usize) {
    let encoded_instruction = EncodedInstruction::from_u8(bytes[0]).unwrap();
    match encoded_instruction {
        EncodedInstruction::Add => (Instruction::Add, 1),
        EncodedInstruction::Sub => (Instruction::Sub, 1),
        EncodedInstruction::Mul => (Instruction::Mul, 1),
        EncodedInstruction::Div => (Instruction::Div, 1),
        EncodedInstruction::IntDiv => (Instruction::IntDiv, 1),
        EncodedInstruction::Mod => (Instruction::Mod, 1),
        EncodedInstruction::Plus => (Instruction::Plus, 1),
        EncodedInstruction::Minus => (Instruction::Minus, 1),
        EncodedInstruction::Concat => (Instruction::Concat, 1),
        EncodedInstruction::Absent => (Instruction::Absent, 1),
        EncodedInstruction::Const => {
            let constant = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Const(constant), 3)
        }
        EncodedInstruction::Closure => {
            let function = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Closure(function), 3)
        }
        EncodedInstruction::NamedTemplate => {
            let function = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::NamedTemplate(function), 3)
        }
        EncodedInstruction::StaticClosure => {
            let function = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::StaticClosure(function), 3)
        }
        EncodedInstruction::Var => {
            let variable = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Var(variable), 3)
        }
        EncodedInstruction::GlobalVar => {
            let variable = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::GlobalVar(variable), 3)
        }
        EncodedInstruction::VarIsAbsent => {
            let variable = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::VarIsAbsent(variable), 3)
        }
        EncodedInstruction::Set => {
            let variable = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Set(variable), 3)
        }
        EncodedInstruction::ClosureVar => {
            let variable = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::ClosureVar(variable), 3)
        }
        EncodedInstruction::Comma => (Instruction::Comma, 1),
        EncodedInstruction::CurlyArray => (Instruction::CurlyArray, 1),
        EncodedInstruction::SquareArray => (Instruction::SquareArray, 1),
        EncodedInstruction::CurlyMap => (Instruction::CurlyMap, 1),
        EncodedInstruction::Eq => (Instruction::Eq, 1),
        EncodedInstruction::Ne => (Instruction::Ne, 1),
        EncodedInstruction::Lt => (Instruction::Lt, 1),
        EncodedInstruction::Le => (Instruction::Le, 1),
        EncodedInstruction::Gt => (Instruction::Gt, 1),
        EncodedInstruction::Ge => (Instruction::Ge, 1),
        EncodedInstruction::GenEq => (Instruction::GenEq, 1),
        EncodedInstruction::GenNe => (Instruction::GenNe, 1),
        EncodedInstruction::GenLt => (Instruction::GenLt, 1),
        EncodedInstruction::GenLe => (Instruction::GenLe, 1),
        EncodedInstruction::GenGt => (Instruction::GenGt, 1),
        EncodedInstruction::GenGe => (Instruction::GenGe, 1),
        EncodedInstruction::Is => (Instruction::Is, 1),
        EncodedInstruction::Precedes => (Instruction::Precedes, 1),
        EncodedInstruction::Follows => (Instruction::Follows, 1),
        EncodedInstruction::Union => (Instruction::Union, 1),
        EncodedInstruction::Intersect => (Instruction::Intersect, 1),
        EncodedInstruction::Except => (Instruction::Except, 1),
        EncodedInstruction::Jump => {
            let displacement = i16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Jump(displacement), 3)
        }
        EncodedInstruction::JumpIfTrue => {
            let displacement = i16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::JumpIfTrue(displacement), 3)
        }
        EncodedInstruction::JumpIfFalse => {
            let displacement = i16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::JumpIfFalse(displacement), 3)
        }
        EncodedInstruction::Call => {
            let arity = bytes[1];
            (Instruction::Call(arity), 2)
        }
        EncodedInstruction::Lookup => (Instruction::Lookup, 1),
        EncodedInstruction::WildcardLookup => (Instruction::WildcardLookup, 1),
        EncodedInstruction::Step => {
            let step = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Step(step), 3)
        }
        EncodedInstruction::Deduplicate => (Instruction::Deduplicate, 1),
        EncodedInstruction::Cast => {
            let type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Cast(type_id), 3)
        }
        EncodedInstruction::Castable => {
            let type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Castable(type_id), 3)
        }
        EncodedInstruction::InstanceOf => {
            let sequence_type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::InstanceOf(sequence_type_id), 3)
        }
        EncodedInstruction::Treat => {
            let sequence_type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::Treat(sequence_type_id), 3)
        }
        EncodedInstruction::Return => (Instruction::Return, 1),
        EncodedInstruction::ReturnConvert => {
            let sequence_type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::ReturnConvert(sequence_type_id), 3)
        }
        EncodedInstruction::ConvertSequence => {
            let sequence_type_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            let error_id = u16::from_le_bytes([bytes[3], bytes[4]]);
            (
                Instruction::ConvertSequence(sequence_type_id, RaisedError::from_u16(error_id)),
                5,
            )
        }
        EncodedInstruction::Dup => (Instruction::Dup, 1),
        EncodedInstruction::Pop => (Instruction::Pop, 1),
        EncodedInstruction::LetDone => (Instruction::LetDone, 1),
        EncodedInstruction::Range => (Instruction::Range, 1),
        EncodedInstruction::SequenceLen => (Instruction::SequenceLen, 1),
        EncodedInstruction::SequenceGet => (Instruction::SequenceGet, 1),
        EncodedInstruction::BuildNew => (Instruction::BuildNew, 1),
        EncodedInstruction::BuildPush => (Instruction::BuildPush, 1),
        EncodedInstruction::BuildComplete => (Instruction::BuildComplete, 1),
        EncodedInstruction::IsNumeric => (Instruction::IsNumeric, 1),
        EncodedInstruction::XmlName => (Instruction::XmlName, 1),
        EncodedInstruction::XmlDocument => (Instruction::XmlDocument, 1),
        EncodedInstruction::XmlElement => (Instruction::XmlElement, 1),
        EncodedInstruction::XmlAttribute => (Instruction::XmlAttribute, 1),
        EncodedInstruction::XmlNamespace => (Instruction::XmlNamespace, 1),
        EncodedInstruction::XmlText => (Instruction::XmlText, 1),
        EncodedInstruction::XmlComment => (Instruction::XmlComment, 1),
        EncodedInstruction::XmlProcessingInstruction => (Instruction::XmlProcessingInstruction, 1),
        EncodedInstruction::XmlAppend => (Instruction::XmlAppend, 1),
        EncodedInstruction::CopyShallow => (Instruction::CopyShallow, 1),
        EncodedInstruction::CopyDeep => (Instruction::CopyDeep, 1),
        EncodedInstruction::CallTemplate => (Instruction::CallTemplate, 1),
        EncodedInstruction::ContinueTemplate => (Instruction::ContinueTemplate, 1),
        EncodedInstruction::ApplyTemplates => {
            let mode_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            let builtin_template_params_passthrough = bytes[3] != 0;
            (
                Instruction::ApplyTemplates(mode_id, builtin_template_params_passthrough),
                4,
            )
        }
        EncodedInstruction::ApplyTemplatesCurrent => {
            let fallback_mode_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            let builtin_template_params_passthrough = bytes[3] != 0;
            (
                Instruction::ApplyTemplatesCurrent(
                    fallback_mode_id,
                    builtin_template_params_passthrough,
                ),
                4,
            )
        }
        EncodedInstruction::RaiseError => {
            let error_id = u16::from_le_bytes([bytes[1], bytes[2]]);
            (Instruction::RaiseError(RaisedError::from_u16(error_id)), 3)
        }
        EncodedInstruction::PrintTop => (Instruction::PrintTop, 1),
        EncodedInstruction::PrintStack => (Instruction::PrintStack, 1),
    }
}

pub fn decode_instructions(bytes: &[u8]) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut ip = 0;
    while ip < bytes.len() {
        let (instruction, instruction_size) = decode_instruction(&bytes[ip..]);
        instructions.push(instruction);
        ip += instruction_size;
    }
    instructions
}

pub fn encode_instruction(instruction: Instruction, bytes: &mut Vec<u8>) {
    match instruction {
        Instruction::Add => bytes.push(EncodedInstruction::Add.to_u8().unwrap()),
        Instruction::Sub => bytes.push(EncodedInstruction::Sub.to_u8().unwrap()),
        Instruction::Mul => bytes.push(EncodedInstruction::Mul.to_u8().unwrap()),
        Instruction::Div => bytes.push(EncodedInstruction::Div.to_u8().unwrap()),
        Instruction::IntDiv => bytes.push(EncodedInstruction::IntDiv.to_u8().unwrap()),
        Instruction::Mod => bytes.push(EncodedInstruction::Mod.to_u8().unwrap()),
        Instruction::Plus => bytes.push(EncodedInstruction::Plus.to_u8().unwrap()),
        Instruction::Minus => bytes.push(EncodedInstruction::Minus.to_u8().unwrap()),
        Instruction::Concat => bytes.push(EncodedInstruction::Concat.to_u8().unwrap()),
        Instruction::Absent => bytes.push(EncodedInstruction::Absent.to_u8().unwrap()),
        Instruction::Const(constant) => {
            bytes.push(EncodedInstruction::Const.to_u8().unwrap());
            bytes.extend_from_slice(&constant.to_le_bytes());
        }
        Instruction::Closure(function_id) => {
            bytes.push(EncodedInstruction::Closure.to_u8().unwrap());
            bytes.extend_from_slice(&function_id.to_le_bytes());
        }
        Instruction::NamedTemplate(function_id) => {
            bytes.push(EncodedInstruction::NamedTemplate.to_u8().unwrap());
            bytes.extend_from_slice(&function_id.to_le_bytes());
        }
        Instruction::StaticClosure(function_id) => {
            bytes.push(EncodedInstruction::StaticClosure.to_u8().unwrap());
            bytes.extend_from_slice(&function_id.to_le_bytes());
        }
        Instruction::Var(variable) => {
            bytes.push(EncodedInstruction::Var.to_u8().unwrap());
            bytes.extend_from_slice(&variable.to_le_bytes());
        }
        Instruction::GlobalVar(variable) => {
            bytes.push(EncodedInstruction::GlobalVar.to_u8().unwrap());
            bytes.extend_from_slice(&variable.to_le_bytes());
        }
        Instruction::VarIsAbsent(variable) => {
            bytes.push(EncodedInstruction::VarIsAbsent.to_u8().unwrap());
            bytes.extend_from_slice(&variable.to_le_bytes());
        }
        Instruction::Set(variable) => {
            bytes.push(EncodedInstruction::Set.to_u8().unwrap());
            bytes.extend_from_slice(&variable.to_le_bytes());
        }
        Instruction::ClosureVar(variable) => {
            bytes.push(EncodedInstruction::ClosureVar.to_u8().unwrap());
            bytes.extend_from_slice(&variable.to_le_bytes());
        }
        Instruction::Comma => bytes.push(EncodedInstruction::Comma.to_u8().unwrap()),
        Instruction::CurlyArray => bytes.push(EncodedInstruction::CurlyArray.to_u8().unwrap()),
        Instruction::SquareArray => bytes.push(EncodedInstruction::SquareArray.to_u8().unwrap()),
        Instruction::CurlyMap => bytes.push(EncodedInstruction::CurlyMap.to_u8().unwrap()),
        Instruction::Eq => bytes.push(EncodedInstruction::Eq.to_u8().unwrap()),
        Instruction::Ne => bytes.push(EncodedInstruction::Ne.to_u8().unwrap()),
        Instruction::Lt => bytes.push(EncodedInstruction::Lt.to_u8().unwrap()),
        Instruction::Le => bytes.push(EncodedInstruction::Le.to_u8().unwrap()),
        Instruction::Gt => bytes.push(EncodedInstruction::Gt.to_u8().unwrap()),
        Instruction::Ge => bytes.push(EncodedInstruction::Ge.to_u8().unwrap()),
        Instruction::GenEq => bytes.push(EncodedInstruction::GenEq.to_u8().unwrap()),
        Instruction::GenNe => bytes.push(EncodedInstruction::GenNe.to_u8().unwrap()),
        Instruction::GenLt => bytes.push(EncodedInstruction::GenLt.to_u8().unwrap()),
        Instruction::GenLe => bytes.push(EncodedInstruction::GenLe.to_u8().unwrap()),
        Instruction::GenGt => bytes.push(EncodedInstruction::GenGt.to_u8().unwrap()),
        Instruction::GenGe => bytes.push(EncodedInstruction::GenGe.to_u8().unwrap()),
        Instruction::Is => bytes.push(EncodedInstruction::Is.to_u8().unwrap()),
        Instruction::Precedes => bytes.push(EncodedInstruction::Precedes.to_u8().unwrap()),
        Instruction::Follows => bytes.push(EncodedInstruction::Follows.to_u8().unwrap()),
        Instruction::Union => bytes.push(EncodedInstruction::Union.to_u8().unwrap()),
        Instruction::Intersect => bytes.push(EncodedInstruction::Intersect.to_u8().unwrap()),
        Instruction::Except => bytes.push(EncodedInstruction::Except.to_u8().unwrap()),
        Instruction::Jump(displacement) => {
            bytes.push(EncodedInstruction::Jump.to_u8().unwrap());
            bytes.extend_from_slice(&displacement.to_le_bytes());
        }
        Instruction::JumpIfTrue(displacement) => {
            bytes.push(EncodedInstruction::JumpIfTrue.to_u8().unwrap());
            bytes.extend_from_slice(&displacement.to_le_bytes());
        }
        Instruction::JumpIfFalse(displacement) => {
            bytes.push(EncodedInstruction::JumpIfFalse.to_u8().unwrap());
            bytes.extend_from_slice(&displacement.to_le_bytes());
        }
        Instruction::Call(arity) => {
            bytes.push(EncodedInstruction::Call.to_u8().unwrap());
            bytes.push(arity);
        }
        Instruction::Lookup => bytes.push(EncodedInstruction::Lookup.to_u8().unwrap()),
        Instruction::WildcardLookup => {
            bytes.push(EncodedInstruction::WildcardLookup.to_u8().unwrap())
        }
        Instruction::Step(step_id) => {
            bytes.push(EncodedInstruction::Step.to_u8().unwrap());
            bytes.extend_from_slice(&step_id.to_le_bytes());
        }
        Instruction::Deduplicate => {
            bytes.push(EncodedInstruction::Deduplicate.to_u8().unwrap());
        }
        Instruction::Return => bytes.push(EncodedInstruction::Return.to_u8().unwrap()),
        Instruction::ReturnConvert(sequence_type_id) => {
            bytes.push(EncodedInstruction::ReturnConvert.to_u8().unwrap());
            bytes.extend_from_slice(&sequence_type_id.to_le_bytes());
        }
        Instruction::ConvertSequence(sequence_type_id, error) => {
            bytes.push(EncodedInstruction::ConvertSequence.to_u8().unwrap());
            bytes.extend_from_slice(&sequence_type_id.to_le_bytes());
            bytes.extend_from_slice(&error.to_u16().to_le_bytes());
        }
        Instruction::Dup => bytes.push(EncodedInstruction::Dup.to_u8().unwrap()),
        Instruction::Pop => bytes.push(EncodedInstruction::Pop.to_u8().unwrap()),
        Instruction::LetDone => bytes.push(EncodedInstruction::LetDone.to_u8().unwrap()),
        Instruction::Cast(type_id) => {
            bytes.push(EncodedInstruction::Cast.to_u8().unwrap());
            bytes.extend_from_slice(&type_id.to_le_bytes());
        }
        Instruction::Castable(type_id) => {
            bytes.push(EncodedInstruction::Castable.to_u8().unwrap());
            bytes.extend_from_slice(&type_id.to_le_bytes());
        }
        Instruction::InstanceOf(sequence_type_id) => {
            bytes.push(EncodedInstruction::InstanceOf.to_u8().unwrap());
            bytes.extend_from_slice(&sequence_type_id.to_le_bytes());
        }
        Instruction::Treat(sequence_type_id) => {
            bytes.push(EncodedInstruction::Treat.to_u8().unwrap());
            bytes.extend_from_slice(&sequence_type_id.to_le_bytes());
        }
        Instruction::Range => bytes.push(EncodedInstruction::Range.to_u8().unwrap()),
        Instruction::SequenceLen => bytes.push(EncodedInstruction::SequenceLen.to_u8().unwrap()),
        Instruction::SequenceGet => bytes.push(EncodedInstruction::SequenceGet.to_u8().unwrap()),
        Instruction::BuildNew => bytes.push(EncodedInstruction::BuildNew.to_u8().unwrap()),
        Instruction::BuildPush => bytes.push(EncodedInstruction::BuildPush.to_u8().unwrap()),
        Instruction::BuildComplete => {
            bytes.push(EncodedInstruction::BuildComplete.to_u8().unwrap())
        }
        Instruction::IsNumeric => bytes.push(EncodedInstruction::IsNumeric.to_u8().unwrap()),
        Instruction::XmlName => bytes.push(EncodedInstruction::XmlName.to_u8().unwrap()),
        Instruction::XmlDocument => bytes.push(EncodedInstruction::XmlDocument.to_u8().unwrap()),
        Instruction::XmlElement => bytes.push(EncodedInstruction::XmlElement.to_u8().unwrap()),
        Instruction::XmlAttribute => bytes.push(EncodedInstruction::XmlAttribute.to_u8().unwrap()),
        Instruction::XmlNamespace => bytes.push(EncodedInstruction::XmlNamespace.to_u8().unwrap()),
        Instruction::XmlText => bytes.push(EncodedInstruction::XmlText.to_u8().unwrap()),
        Instruction::XmlComment => bytes.push(EncodedInstruction::XmlComment.to_u8().unwrap()),
        Instruction::XmlProcessingInstruction => bytes.push(
            EncodedInstruction::XmlProcessingInstruction
                .to_u8()
                .unwrap(),
        ),
        Instruction::XmlAppend => bytes.push(EncodedInstruction::XmlAppend.to_u8().unwrap()),
        Instruction::CopyShallow => bytes.push(EncodedInstruction::CopyShallow.to_u8().unwrap()),
        Instruction::CopyDeep => bytes.push(EncodedInstruction::CopyDeep.to_u8().unwrap()),
        Instruction::CallTemplate => bytes.push(EncodedInstruction::CallTemplate.to_u8().unwrap()),
        Instruction::ContinueTemplate => {
            bytes.push(EncodedInstruction::ContinueTemplate.to_u8().unwrap())
        }
        Instruction::ApplyTemplates(mode_id, builtin_template_params_passthrough) => {
            bytes.push(EncodedInstruction::ApplyTemplates.to_u8().unwrap());
            bytes.extend_from_slice(&mode_id.to_le_bytes());
            bytes.push(u8::from(builtin_template_params_passthrough));
        }
        Instruction::ApplyTemplatesCurrent(
            fallback_mode_id,
            builtin_template_params_passthrough,
        ) => {
            bytes.push(EncodedInstruction::ApplyTemplatesCurrent.to_u8().unwrap());
            bytes.extend_from_slice(&fallback_mode_id.to_le_bytes());
            bytes.push(u8::from(builtin_template_params_passthrough));
        }
        Instruction::RaiseError(error) => {
            bytes.push(EncodedInstruction::RaiseError.to_u8().unwrap());
            bytes.extend_from_slice(&error.to_u16().to_le_bytes());
        }
        Instruction::PrintTop => bytes.push(EncodedInstruction::PrintTop.to_u8().unwrap()),
        Instruction::PrintStack => bytes.push(EncodedInstruction::PrintStack.to_u8().unwrap()),
    }
}

pub(crate) fn encode_instructions(instructions: Vec<Instruction>, bytes: &mut Vec<u8>) {
    for instruction in instructions {
        encode_instruction(instruction, bytes);
    }
}

// size in bytes for an instruction
pub fn instruction_size(instruction: &Instruction) -> usize {
    match instruction {
        Instruction::Add
        | Instruction::Sub
        | Instruction::Mul
        | Instruction::Div
        | Instruction::IntDiv
        | Instruction::Mod
        | Instruction::Plus
        | Instruction::Minus
        | Instruction::Concat
        | Instruction::Absent
        | Instruction::Comma
        | Instruction::CurlyArray
        | Instruction::SquareArray
        | Instruction::CurlyMap
        | Instruction::Eq
        | Instruction::Ne
        | Instruction::Lt
        | Instruction::Le
        | Instruction::Gt
        | Instruction::Ge
        | Instruction::GenEq
        | Instruction::GenNe
        | Instruction::GenLt
        | Instruction::GenLe
        | Instruction::GenGt
        | Instruction::GenGe
        | Instruction::Is
        | Instruction::Precedes
        | Instruction::Follows
        | Instruction::Union
        | Instruction::Intersect
        | Instruction::Except
        | Instruction::Return
        | Instruction::Dup
        | Instruction::Pop
        | Instruction::LetDone
        | Instruction::Range
        | Instruction::SequenceLen
        | Instruction::SequenceGet
        | Instruction::BuildNew
        | Instruction::BuildPush
        | Instruction::BuildComplete
        | Instruction::IsNumeric
        | Instruction::Deduplicate
        | Instruction::Lookup
        | Instruction::WildcardLookup
        | Instruction::XmlName
        | Instruction::XmlDocument
        | Instruction::XmlElement
        | Instruction::XmlAttribute
        | Instruction::XmlNamespace
        | Instruction::XmlText
        | Instruction::XmlComment
        | Instruction::XmlProcessingInstruction
        | Instruction::XmlAppend
        | Instruction::CopyShallow
        | Instruction::CopyDeep
        | Instruction::CallTemplate
        | Instruction::ContinueTemplate
        | Instruction::PrintTop
        | Instruction::PrintStack => 1,
        Instruction::Call(_) => 2,
        Instruction::Const(_)
        | Instruction::Closure(_)
        | Instruction::NamedTemplate(_)
        | Instruction::StaticClosure(_)
        | Instruction::Var(_)
        | Instruction::GlobalVar(_)
        | Instruction::VarIsAbsent(_)
        | Instruction::Set(_)
        | Instruction::ClosureVar(_)
        | Instruction::Jump(_)
        | Instruction::JumpIfTrue(_)
        | Instruction::Step(_)
        | Instruction::Cast(_)
        | Instruction::Castable(_)
        | Instruction::InstanceOf(_)
        | Instruction::Treat(_)
        | Instruction::ReturnConvert(_)
        | Instruction::JumpIfFalse(_)
        | Instruction::RaiseError(_) => 3,
        Instruction::ApplyTemplates(_, _) | Instruction::ApplyTemplatesCurrent(_, _) => 4,
        Instruction::ConvertSequence(_, _) => 5,
    }
}

pub(crate) fn read_instruction(bytes: &[u8], ip: &mut usize) -> EncodedInstruction {
    let byte = bytes[*ip];
    *ip += 1;
    EncodedInstruction::from_u8(byte).unwrap()
}

pub(crate) fn read_u16(bytes: &[u8], ip: &mut usize) -> u16 {
    let bytes = &bytes[*ip..*ip + 2];
    *ip += 2;
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub(crate) fn read_i16(bytes: &[u8], ip: &mut usize) -> i16 {
    let bytes = &bytes[*ip..*ip + 2];
    *ip += 2;
    i16::from_le_bytes([bytes[0], bytes[1]])
}

pub(crate) fn read_u8(bytes: &[u8], ip: &mut usize) -> u8 {
    let byte = bytes[*ip];
    *ip += 1;
    byte
}

use crate::function::InlineFunction;

impl InlineFunction {
    pub fn decoded(&self) -> Vec<Instruction> {
        decode_instructions(&self.chunk)
    }
}
