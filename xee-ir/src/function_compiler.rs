use ibig::{ibig, IBig};

use xee_interpreter::error::Error;
use xee_interpreter::function::FunctionRule;
use xee_interpreter::interpreter::instruction::Instruction;
use xee_interpreter::span::SourceSpan;
use xee_interpreter::{error, function, sequence};

use crate::declaration_compiler::ModeIds;
use crate::ir;

use super::builder::{BackwardJumpRef, ForwardJumpRef, FunctionBuilder, JumpCondition};
use super::scope;

pub(crate) type Scopes = scope::Scopes<ir::Name>;

pub struct FunctionCompiler<'a> {
    pub(crate) scopes: &'a mut Scopes,
    pub(crate) mode_ids: &'a ModeIds,
    pub(crate) builder: FunctionBuilder<'a>,
}

impl<'a> FunctionCompiler<'a> {
    pub fn new(
        builder: FunctionBuilder<'a>,
        scopes: &'a mut Scopes,
        mode_ids: &'a ModeIds,
    ) -> Self {
        Self {
            builder,
            scopes,
            mode_ids,
        }
    }

    pub fn compile_expr(&mut self, expr: &ir::ExprS) -> error::SpannedResult<()> {
        let span = expr.span.into();
        match &expr.value {
            ir::Expr::Atom(atom) => self.compile_atom(atom),
            ir::Expr::Let(let_) => self.compile_let(let_, span),
            ir::Expr::Binary(binary) => self.compile_binary(binary, span),
            ir::Expr::Unary(unary) => self.compile_unary(unary, span),
            ir::Expr::FunctionDefinition(function_definition) => {
                self.compile_function_definition(function_definition, span)
            }
            ir::Expr::FunctionCall(function_call) => {
                self.compile_function_call(function_call, span)
            }
            ir::Expr::Lookup(lookup) => self.compile_lookup(lookup, span),
            ir::Expr::WildcardLookup(wildcard_lookup) => {
                self.compile_wildcard_lookup(wildcard_lookup, span)
            }
            ir::Expr::Step(step) => self.compile_step(step, span),
            ir::Expr::Deduplicate(expr) => self.compile_deduplicate(expr, span),
            ir::Expr::If(if_) => self.compile_if(if_, span),
            ir::Expr::Map(map) => self.compile_map(map, span),
            ir::Expr::Filter(filter) => self.compile_filter(filter, span),
            ir::Expr::PatternPredicate(pattern_predicate) => {
                self.compile_pattern_predicate(pattern_predicate, span)
            }
            ir::Expr::Quantified(quantified) => self.compile_quantified(quantified, span),
            ir::Expr::Cast(cast) => self.compile_cast(cast, span),
            ir::Expr::Castable(castable) => self.compile_castable(castable, span),
            ir::Expr::InstanceOf(instance_of) => self.compile_instance_of(instance_of, span),
            ir::Expr::Treat(treat) => self.compile_treat(treat, span),
            ir::Expr::MapConstructor(map_constructor) => {
                self.compile_map_constructor(map_constructor, span)
            }
            ir::Expr::ArrayConstructor(array_constructor) => {
                self.compile_array_constructor(array_constructor, span)
            }
            ir::Expr::XmlName(xml_name) => self.compile_xml_name(xml_name, span),
            ir::Expr::XmlDocument(root) => self.compile_xml_document(root, span),
            ir::Expr::XmlElement(element) => self.compile_xml_element(element, span),
            ir::Expr::XmlAttribute(attribute) => self.compile_xml_attribute(attribute, span),
            ir::Expr::XmlNamespace(namespace) => self.compile_xml_namespace(namespace, span),
            ir::Expr::XmlText(text) => self.compile_xml_text(text, span),
            ir::Expr::XmlComment(comment) => self.compile_xml_comment(comment, span),
            ir::Expr::XmlProcessingInstruction(processing_instruction) => {
                self.compile_xml_processing_instruction(processing_instruction, span)
            }
            ir::Expr::XmlAppend(xml_append) => self.compile_xml_append(xml_append, span),
            ir::Expr::ApplyTemplates(apply_templates) => {
                self.compile_apply_templates(apply_templates, span)
            }
            ir::Expr::CopyShallow(copy_shallow) => self.compile_copy_shallow(copy_shallow, span),
            ir::Expr::CopyDeep(copy_deep) => self.compile_copy_deep(copy_deep, span),
        }
    }

    fn compile_atom(&mut self, atom: &ir::AtomS) -> error::SpannedResult<()> {
        let span = atom.span.into();
        match &atom.value {
            ir::Atom::Const(c) => {
                match c {
                    ir::Const::Integer(i) => {
                        self.builder.emit_constant((i.clone()).into(), span);
                    }
                    ir::Const::String(s) => {
                        self.builder.emit_constant((s).into(), span);
                    }
                    ir::Const::Double(d) => {
                        self.builder.emit_constant((*d).into(), span);
                    }
                    ir::Const::Decimal(d) => {
                        self.builder.emit_constant((*d).into(), span);
                    }
                    ir::Const::EmptySequence => self
                        .builder
                        .emit_constant(sequence::Sequence::default(), span),
                    ir::Const::StaticFunctionReference(static_function_id, context_names) => {
                        self.compile_static_function_reference(
                            *static_function_id,
                            context_names.as_ref(),
                            span,
                        )?;
                    }
                };
                Ok(())
            }
            ir::Atom::Variable(name) => self.compile_variable(name, span),
        }
    }

    fn compile_variable(&mut self, name: &ir::Name, span: SourceSpan) -> error::SpannedResult<()> {
        if let Some(index) = self.scopes.get(name) {
            if index > u16::MAX as usize {
                return Err(Error::XPDY0130.with_span(span));
            }
            self.builder.emit(Instruction::Var(index as u16), span);
            Ok(())
        } else {
            // if value is in any outer scopes
            if self.scopes.is_closed_over_name(name) {
                let index = self.builder.add_closure_name(name);
                if index > u16::MAX as usize {
                    return Err(Error::XPDY0130.with_span(span));
                }
                self.builder
                    .emit(Instruction::ClosureVar(index as u16), span);
                Ok(())
            } else {
                unreachable!("variable not found: {:?}", name);
            }
        }
    }

    fn compile_variable_set(
        &mut self,
        name: &ir::Name,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        if let Some(index) = self.scopes.get(name) {
            if index > u16::MAX as usize {
                return Err(Error::XPDY0130.with_span(span));
            }
            self.builder.emit(Instruction::Set(index as u16), span);
        } else {
            panic!("can only set locals: {:?}", name);
        }
        Ok(())
    }

    fn compile_let(&mut self, let_: &ir::Let, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_expr(&let_.var_expr)?;
        self.scopes.push_name(&let_.name);
        self.compile_expr(&let_.return_expr)?;
        self.builder.emit(Instruction::LetDone, span);
        self.scopes.pop_name();
        Ok(())
    }

    fn compile_if(&mut self, if_: &ir::If, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_atom(&if_.condition)?;
        let jump_else = self.builder.emit_jump_forward(JumpCondition::False, span);
        self.compile_expr(&if_.then)?;
        let jump_end = self.builder.emit_jump_forward(JumpCondition::Always, span);
        self.builder.patch_jump(jump_else);
        self.compile_expr(&if_.else_)?;
        self.builder.patch_jump(jump_end);
        Ok(())
    }

    fn compile_binary(
        &mut self,
        binary: &ir::Binary,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&binary.left)?;
        self.compile_atom(&binary.right)?;
        match &binary.op {
            ir::BinaryOperator::Add => {
                self.builder.emit(Instruction::Add, span);
            }
            ir::BinaryOperator::Sub => {
                self.builder.emit(Instruction::Sub, span);
            }
            ir::BinaryOperator::Mul => {
                self.builder.emit(Instruction::Mul, span);
            }
            ir::BinaryOperator::Div => {
                self.builder.emit(Instruction::Div, span);
            }
            ir::BinaryOperator::IntDiv => {
                self.builder.emit(Instruction::IntDiv, span);
            }
            ir::BinaryOperator::Mod => {
                self.builder.emit(Instruction::Mod, span);
            }
            ir::BinaryOperator::ValueEq => {
                self.builder.emit(Instruction::Eq, span);
            }
            ir::BinaryOperator::ValueNe => {
                self.builder.emit(Instruction::Ne, span);
            }
            ir::BinaryOperator::ValueLt => {
                self.builder.emit(Instruction::Lt, span);
            }
            ir::BinaryOperator::ValueLe => {
                self.builder.emit(Instruction::Le, span);
            }
            ir::BinaryOperator::ValueGt => {
                self.builder.emit(Instruction::Gt, span);
            }
            ir::BinaryOperator::ValueGe => {
                self.builder.emit(Instruction::Ge, span);
            }
            ir::BinaryOperator::GenEq => {
                self.builder.emit(Instruction::GenEq, span);
            }
            ir::BinaryOperator::GenNe => {
                self.builder.emit(Instruction::GenNe, span);
            }
            ir::BinaryOperator::GenLt => {
                self.builder.emit(Instruction::GenLt, span);
            }
            ir::BinaryOperator::GenLe => {
                self.builder.emit(Instruction::GenLe, span);
            }
            ir::BinaryOperator::GenGt => {
                self.builder.emit(Instruction::GenGt, span);
            }
            ir::BinaryOperator::GenGe => {
                self.builder.emit(Instruction::GenGe, span);
            }
            ir::BinaryOperator::Comma => {
                self.builder.emit(Instruction::Comma, span);
            }
            ir::BinaryOperator::Union => {
                self.builder.emit(Instruction::Union, span);
            }
            ir::BinaryOperator::Intersect => {
                self.builder.emit(Instruction::Intersect, span);
            }
            ir::BinaryOperator::Except => {
                self.builder.emit(Instruction::Except, span);
            }
            ir::BinaryOperator::Range => {
                self.builder.emit(Instruction::Range, span);
            }
            ir::BinaryOperator::Concat => {
                self.builder.emit(Instruction::Concat, span);
            }
            ir::BinaryOperator::And => {
                // XXX we don't do any short-circuiting of evaluation yet
                let first_false = self.builder.emit_jump_forward(JumpCondition::False, span);
                let second_false = self.builder.emit_jump_forward(JumpCondition::False, span);
                // both are true, so put true on stack and jump to end
                self.builder.emit_constant(true.into(), span);
                let end = self.builder.emit_jump_forward(JumpCondition::Always, span);
                self.builder.patch_jump(first_false);
                // pop the second item on the stack
                self.builder.emit(Instruction::Pop, span);
                self.builder.patch_jump(second_false);
                // now put false on the stack
                self.builder.emit_constant(false.into(), span);
                self.builder.patch_jump(end);
            }
            ir::BinaryOperator::Or => {
                // XXX we don't do any short-circuiting of evaluation yet
                let first_true = self.builder.emit_jump_forward(JumpCondition::True, span);
                let second_true = self.builder.emit_jump_forward(JumpCondition::True, span);
                // both are false, so put false on stack and jump to end
                self.builder.emit_constant(false.into(), span);
                let end = self.builder.emit_jump_forward(JumpCondition::Always, span);
                // if first is true, pop second
                self.builder.patch_jump(first_true);
                // pop the second item on the stack
                self.builder.emit(Instruction::Pop, span);
                self.builder.patch_jump(second_true);
                // now put true on the stack
                self.builder.emit_constant(true.into(), span);
                self.builder.patch_jump(end);
            }
            ir::BinaryOperator::Is => {
                self.builder.emit(Instruction::Is, span);
            }
            ir::BinaryOperator::Precedes => {
                self.builder.emit(Instruction::Precedes, span);
            }
            ir::BinaryOperator::Follows => {
                self.builder.emit(Instruction::Follows, span);
            }
        }
        Ok(())
    }

    fn compile_unary(&mut self, unary: &ir::Unary, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_atom(&unary.atom)?;
        match unary.op {
            ir::UnaryOperator::Plus => {
                self.builder.emit(Instruction::Plus, span);
            }
            ir::UnaryOperator::Minus => {
                self.builder.emit(Instruction::Minus, span);
            }
        }
        Ok(())
    }

    pub fn compile_function_id(
        &mut self,
        function_definition: &ir::FunctionDefinition,
        span: SourceSpan,
    ) -> error::SpannedResult<function::InlineFunctionId> {
        let nested_builder = self.builder.builder();
        self.scopes.push_scope();

        let mut compiler = FunctionCompiler {
            builder: nested_builder,
            scopes: self.scopes,
            mode_ids: self.mode_ids,
        };

        for param in &function_definition.params {
            compiler.scopes.push_name(&param.name);
        }
        compiler.compile_expr(&function_definition.body)?;
        for _ in &function_definition.params {
            compiler.scopes.pop_name();
        }

        compiler.scopes.pop_scope();

        let function = compiler
            .builder
            .finish("inline".to_string(), function_definition, span);
        // now place all captured names on stack, to ensure we have the
        // closure
        // in reverse order so we can pop them off in the right order
        for name in function.closure_names.iter().rev() {
            self.compile_variable(name, span)?;
        }
        Ok(self.builder.add_function(function))
    }

    pub(crate) fn compile_function_definition(
        &mut self,
        function_definition: &ir::FunctionDefinition,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        let function_id = self.compile_function_id(function_definition, span)?;
        self.builder
            .emit(Instruction::Closure(function_id.as_u16()), span);
        Ok(())
    }

    fn compile_static_function_reference(
        &mut self,
        static_function_id: function::StaticFunctionId,
        context_names: Option<&ir::ContextNames>,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        let static_function = self
            .builder
            .static_context()
            .function_by_id(static_function_id);
        match static_function.function_rule {
            Some(FunctionRule::ItemFirst) => {
                let context_names = context_names.ok_or(Error::XPDY0002.with_span(span))?;
                self.compile_variable(&context_names.item, span)?
            }
            Some(FunctionRule::ItemLast) => {
                let context_names = context_names.ok_or(Error::XPDY0002.with_span(span))?;
                self.compile_variable(&context_names.item, span)?
            }
            Some(FunctionRule::ItemLastOptional) => {
                if let Some(context_names) = context_names {
                    self.compile_variable(&context_names.item, span)?;
                } else {
                    self.builder
                        .emit_constant(sequence::Sequence::default(), span);
                }
            }
            Some(FunctionRule::PositionFirst) => self.compile_variable(
                {
                    let context_names = context_names.ok_or(Error::XPDY0002.with_span(span))?;
                    &context_names.position
                },
                span,
            )?,
            Some(FunctionRule::SizeFirst) => {
                let context_names = context_names.ok_or(Error::XPDY0002.with_span(span))?;
                self.compile_variable(&context_names.last, span)?
            }
            Some(FunctionRule::Collation) | Some(FunctionRule::AnonymousClosure) | None => {}
        }
        self.builder.emit(
            Instruction::StaticClosure(static_function_id.as_u16()),
            span,
        );
        Ok(())
    }

    fn compile_function_call(
        &mut self,
        function_call: &ir::FunctionCall,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&function_call.atom)?;
        for arg in &function_call.args {
            self.compile_atom(arg)?;
        }
        self.builder
            .emit(Instruction::Call(function_call.args.len() as u8), span);
        Ok(())
    }

    fn compile_lookup(
        &mut self,
        lookup: &ir::Lookup,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&lookup.atom)?;
        self.compile_atom(&lookup.arg_atom)?;
        self.builder.emit(Instruction::Lookup, span);
        Ok(())
    }

    fn compile_wildcard_lookup(
        &mut self,
        lookup: &ir::WildcardLookup,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&lookup.atom)?;
        self.builder.emit(Instruction::WildcardLookup, span);
        Ok(())
    }

    fn compile_step(&mut self, step: &ir::Step, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_atom(&step.context)?;
        let step_id = self.builder.add_step(step.step.clone());
        self.builder.emit(Instruction::Step(step_id as u16), span);
        Ok(())
    }

    fn compile_deduplicate(
        &mut self,
        expr: &ir::ExprS,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_expr(expr)?;
        self.builder.emit(Instruction::Deduplicate, span);
        Ok(())
    }

    fn compile_cast(&mut self, cast: &ir::Cast, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_atom(&cast.atom)?;
        let cast_type = cast.cast_type();
        let cast_type_id = self.builder.add_cast_type(cast_type);
        self.builder
            .emit(Instruction::Cast(cast_type_id as u16), span);
        Ok(())
    }

    fn compile_castable(
        &mut self,
        castable: &ir::Castable,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&castable.atom)?;
        let cast_type = castable.cast_type();
        let cast_type_id = self.builder.add_cast_type(cast_type);
        self.builder
            .emit(Instruction::Castable(cast_type_id as u16), span);
        Ok(())
    }

    fn compile_instance_of(
        &mut self,
        instance_of: &ir::InstanceOf,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&instance_of.atom)?;
        let sequence_type_id = self
            .builder
            .add_sequence_type(instance_of.sequence_type.clone());
        self.builder
            .emit(Instruction::InstanceOf(sequence_type_id as u16), span);
        Ok(())
    }

    fn compile_treat(&mut self, treat: &ir::Treat, span: SourceSpan) -> error::SpannedResult<()> {
        self.compile_atom(&treat.atom)?;
        let sequence_type_id = self.builder.add_sequence_type(treat.sequence_type.clone());
        self.builder
            .emit(Instruction::Treat(sequence_type_id as u16), span);
        Ok(())
    }

    fn compile_map_constructor(
        &mut self,
        map_constructor: &ir::MapConstructor,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // compile them in reverse, so we can pop them in the right
        // order during runtime. It matters less with a map, but may
        // still be important for consistent duplicate key detection.
        for (key_atom, value_atom) in map_constructor.members.iter().rev() {
            self.compile_atom(key_atom)?;
            self.compile_atom(value_atom)?;
        }
        // emit constant with size of map
        let len: IBig = map_constructor.members.len().into();
        let len: sequence::Sequence = len.into();
        self.builder.emit_constant(len, span);
        self.builder.emit(Instruction::CurlyMap, span);
        Ok(())
    }

    fn compile_array_constructor(
        &mut self,
        array_constructor: &ir::ArrayConstructor,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        match array_constructor {
            ir::ArrayConstructor::Curly(atom) => {
                self.compile_curly_array_constructor(atom, span)?;
            }
            ir::ArrayConstructor::Square(atoms) => {
                self.compile_square_array_constructor(atoms, span)?;
            }
        }
        Ok(())
    }

    fn compile_curly_array_constructor(
        &mut self,
        atom: &ir::AtomS,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(atom)?;
        self.builder.emit(Instruction::CurlyArray, span);
        Ok(())
    }

    fn compile_square_array_constructor(
        &mut self,
        atoms: &[ir::AtomS],
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // compile them in reverse, so we can pop them in the right
        // order during runtime
        for atom in atoms.iter().rev() {
            self.compile_atom(atom)?;
        }
        // emit constant with length of array
        let len: IBig = atoms.len().into();
        let len: sequence::Sequence = len.into();
        self.builder.emit_constant(len, span);
        self.builder.emit(Instruction::SquareArray, span);
        Ok(())
    }

    fn compile_map(&mut self, map: &ir::Map, span: SourceSpan) -> error::SpannedResult<()> {
        // create new build sequence on build stack
        self.builder.emit(Instruction::BuildNew, span);

        let (loop_start, loop_end) =
            self.compile_sequence_loop_init(&map.var_atom, &map.context_names, span)?;

        self.compile_sequence_get_item(&map.var_atom, &map.context_names, span)?;
        // name it
        self.scopes.push_name(&map.context_names.item);
        // execute the map expression, placing result on stack
        self.compile_expr(&map.return_expr)?;
        self.scopes.pop_name();

        // push result to build
        self.builder.emit(Instruction::BuildPush, span);

        // clean up the var_name item
        self.builder.emit(Instruction::Pop, span);

        self.compile_sequence_loop_iterate(loop_start, &map.context_names, span)?;

        self.builder.patch_jump(loop_end);
        self.compile_sequence_loop_end(span);

        self.builder.emit(Instruction::BuildComplete, span);
        // pop sequence length name & index;
        self.scopes.pop_name();
        self.scopes.pop_name();
        Ok(())
    }

    fn compile_filter(
        &mut self,
        filter: &ir::Filter,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // create new build sequence on build stack
        self.builder.emit(Instruction::BuildNew, span);

        let (loop_start, loop_end) =
            self.compile_sequence_loop_init(&filter.var_atom, &filter.context_names, span)?;

        // place item to filter on stack
        self.compile_sequence_get_item(&filter.var_atom, &filter.context_names, span)?;
        // name it
        self.scopes.push_name(&filter.context_names.item);
        // execute the filter expression, placing result on stack
        self.compile_expr(&filter.return_expr)?;
        self.scopes.pop_name();
        // duplicate result so we can do the IsNumeric check
        self.builder.emit(Instruction::Dup, span);
        // the resulting value can be a numeric value
        self.builder.emit(Instruction::IsNumeric, span);
        // if it's not a numeric expression we're going to interpret it as boolean,
        // a normal filter
        let is_not_numeric = self.builder.emit_jump_forward(JumpCondition::False, span);
        // It was numeric, we have on the stack a position to compare with
        self.compile_variable(&filter.context_names.position, span)?;
        self.builder.emit(Instruction::Eq, span);
        // Now we have a boolean on the stack: a normal filter

        // We take the effective boolean value of the result
        // if filter is false, we skip this item
        self.builder.patch_jump(is_not_numeric);
        let is_included = self.builder.emit_jump_forward(JumpCondition::True, span);
        // we need to clean up the stack after this
        self.builder.emit(Instruction::Pop, span);
        // and iterate the loop
        let iterate = self.builder.emit_jump_forward(JumpCondition::Always, span);

        self.builder.patch_jump(is_included);
        // push item to new build
        self.builder.emit(Instruction::BuildPush, span);

        self.builder.patch_jump(iterate);
        // no need to clean up the stack, as filter get is pushed onto sequence
        self.compile_sequence_loop_iterate(loop_start, &filter.context_names, span)?;

        self.builder.patch_jump(loop_end);
        self.compile_sequence_loop_end(span);

        self.builder.emit(Instruction::BuildComplete, span);
        // pop new sequence length name & index
        self.scopes.pop_name();
        self.scopes.pop_name();
        Ok(())
    }

    fn compile_quantified(
        &mut self,
        quantified: &ir::Quantified,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        let (loop_start, loop_end) =
            self.compile_sequence_loop_init(&quantified.var_atom, &quantified.context_names, span)?;

        self.compile_sequence_get_item(&quantified.var_atom, &quantified.context_names, span)?;
        // name it
        self.scopes.push_name(&quantified.context_names.item);
        // execute the satisfies expression, placing result in on stack
        self.compile_expr(&quantified.satisifies_expr)?;
        self.scopes.pop_name();

        let jump_out_end = match quantified.quantifier {
            ir::Quantifier::Some => self.builder.emit_jump_forward(JumpCondition::True, span),
            ir::Quantifier::Every => self.builder.emit_jump_forward(JumpCondition::False, span),
        };
        // we didn't jump out, clean up quantifier variable
        self.builder.emit(Instruction::Pop, span);

        self.compile_sequence_loop_iterate(loop_start, &quantified.context_names, span)?;

        self.builder.patch_jump(loop_end);

        // if we reached the end, without jumping out
        self.compile_sequence_loop_end(span);

        let reached_end_value = match quantified.quantifier {
            ir::Quantifier::Some => false.into(),
            ir::Quantifier::Every => true.into(),
        };
        self.builder.emit_constant(reached_end_value, span);
        let end = self.builder.emit_jump_forward(JumpCondition::Always, span);

        // we jumped out
        self.builder.patch_jump(jump_out_end);
        // clean up quantifier variable
        self.builder.emit(Instruction::Pop, span);
        self.compile_sequence_loop_end(span);

        let jumped_out_value = match quantified.quantifier {
            ir::Quantifier::Some => true.into(),
            ir::Quantifier::Every => false.into(),
        };
        // if we jumped out, we set satisfies to true
        self.builder.emit_constant(jumped_out_value, span);

        self.builder.patch_jump(end);
        // pop sequence length name & index
        self.scopes.pop_name();
        self.scopes.pop_name();
        Ok(())
    }

    fn compile_sequence_loop_init(
        &mut self,
        atom: &ir::AtomS,
        context_names: &ir::ContextNames,
        span: SourceSpan,
    ) -> error::SpannedResult<(BackwardJumpRef, ForwardJumpRef)> {
        //  sequence length
        self.compile_atom(atom)?;
        self.scopes.push_name(&context_names.last);
        self.builder.emit(Instruction::SequenceLen, span);

        // place index on stack
        self.builder.emit_constant(ibig!(1).into(), span);
        self.scopes.push_name(&context_names.position);

        let loop_start_ref = self.builder.loop_start();

        // compare with sequence length, if index is gt length, we're done with the loop
        self.compile_variable(&context_names.position, span)?;
        self.compile_variable(&context_names.last, span)?;
        self.builder.emit(Instruction::Gt, span);
        // check whether index is gt length, if so, we're done with the loop
        let loop_end_ref = self.builder.emit_jump_forward(JumpCondition::True, span);

        Ok((loop_start_ref, loop_end_ref))
    }

    fn compile_sequence_get_item(
        &mut self,
        atom: &ir::AtomS,
        context_names: &ir::ContextNames,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // get item at the index
        self.compile_variable(&context_names.position, span)?;
        self.compile_atom(atom)?;
        self.builder.emit(Instruction::SequenceGet, span);
        Ok(())
    }

    fn compile_sequence_loop_iterate(
        &mut self,
        loop_start: BackwardJumpRef,
        context_names: &ir::ContextNames,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // update index with 1
        self.compile_variable(&context_names.position, span)?;
        self.builder.emit_constant(ibig!(1).into(), span);
        self.builder.emit(Instruction::Add, span);
        self.compile_variable_set(&context_names.position, span)?;
        self.builder
            .emit_jump_backward(loop_start, JumpCondition::Always, span);
        Ok(())
    }

    fn compile_sequence_loop_end(&mut self, span: SourceSpan) {
        // pop length and index
        self.builder.emit(Instruction::Pop, span);
        self.builder.emit(Instruction::Pop, span);
    }

    fn compile_xml_name(
        &mut self,
        xml_name: &ir::XmlName,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&xml_name.namespace)?;
        self.compile_atom(&xml_name.local_name)?;
        self.builder.emit(Instruction::XmlName, span);
        Ok(())
    }

    fn compile_xml_document(
        &mut self,
        _root: &ir::XmlRoot,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.builder.emit(Instruction::XmlDocument, span);
        Ok(())
    }

    fn compile_xml_element(
        &mut self,
        element: &ir::XmlElement,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&element.name)?;
        self.builder.emit(Instruction::XmlElement, span);
        Ok(())
    }

    fn compile_xml_attribute(
        &mut self,
        attribute: &ir::XmlAttribute,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&attribute.name)?;
        self.compile_atom(&attribute.value)?;
        self.builder.emit(Instruction::XmlAttribute, span);
        Ok(())
    }

    fn compile_xml_namespace(
        &mut self,
        prefix: &ir::XmlNamespace,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&prefix.prefix)?;
        self.compile_atom(&prefix.namespace)?;
        self.builder.emit(Instruction::XmlNamespace, span);
        Ok(())
    }

    fn compile_xml_text(
        &mut self,
        text: &ir::XmlText,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // self.compile_atom(&text.element)?;
        self.compile_atom(&text.value)?;
        self.builder.emit(Instruction::XmlText, span);
        Ok(())
    }

    fn compile_xml_append(
        &mut self,
        append: &ir::XmlAppend,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&append.parent)?;
        self.compile_atom(&append.child)?;
        self.builder.emit(Instruction::XmlAppend, span);
        Ok(())
    }

    fn compile_xml_comment(
        &mut self,
        comment: &ir::XmlComment,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&comment.value)?;
        self.builder.emit(Instruction::XmlComment, span);
        Ok(())
    }

    fn compile_xml_processing_instruction(
        &mut self,
        processing_instruction: &ir::XmlProcessingInstruction,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&processing_instruction.target)?;
        self.compile_atom(&processing_instruction.content)?;
        self.builder
            .emit(Instruction::XmlProcessingInstruction, span);
        Ok(())
    }

    fn compile_apply_templates(
        &mut self,
        apply_templates: &ir::ApplyTemplates,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&apply_templates.select)?;

        let mode_id = if matches!(
            apply_templates.mode,
            ir::ApplyTemplatesModeValue::Named(_) | ir::ApplyTemplatesModeValue::Unnamed
        ) {
            self.mode_ids.get(&apply_templates.mode)
        } else {
            todo!("#current mode not handled yet")
        };
        if let Some(mode_id) = mode_id {
            self.builder
                .emit(Instruction::ApplyTemplates(mode_id.get() as u16), span);
        } else {
            // the mode was never used by any templates, so compile the empty
            // sequence
            self.builder
                .emit_constant(sequence::Sequence::default(), span);
        }
        Ok(())
    }

    fn compile_copy_shallow(
        &mut self,
        copy_shallow: &ir::CopyShallow,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&copy_shallow.select)?;
        self.builder.emit(Instruction::CopyShallow, span);
        Ok(())
    }

    fn compile_copy_deep(
        &mut self,
        copy_deep: &ir::CopyDeep,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        self.compile_atom(&copy_deep.select)?;
        self.builder.emit(Instruction::CopyDeep, span);
        Ok(())
    }

    fn compile_pattern_predicate(
        &mut self,
        predicate: &ir::PatternPredicate,
        span: SourceSpan,
    ) -> error::SpannedResult<()> {
        // execute the expression, placing result on stack
        self.compile_expr(&predicate.expr)?;
        // duplicate result so we can do the IsNumeric check
        self.builder.emit(Instruction::Dup, span);
        // the resulting value can be a numeric value
        self.builder.emit(Instruction::IsNumeric, span);
        // if it's not a numeric expression we're going to interpret it as boolean,
        // a normal filter
        let is_not_numeric = self.builder.emit_jump_forward(JumpCondition::False, span);
        // It was numeric, we have on the stack a position to compare with
        self.compile_variable(&predicate.context_names.position, span)?;
        self.builder.emit(Instruction::Eq, span);
        self.builder.patch_jump(is_not_numeric);
        Ok(())
    }
}
