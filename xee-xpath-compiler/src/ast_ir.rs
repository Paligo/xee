use xee_interpreter::{context, error, error::Error, function, xml};
use xee_ir::{ir, ir::AtomS, Binding, Bindings, Variables};
use xee_schema_type::Xs;
use xee_xpath_ast::{ast, ast::Span, span::Spanned, FN_NAMESPACE};
use xot::xmlname::NameStrInfo;

#[derive(Debug)]
pub struct IrConverter<'a> {
    variables: &'a mut Variables,
    static_context: &'a context::StaticContext,
    fn_position: ast::Name,
    fn_last: ast::Name,
}

impl<'a> IrConverter<'a> {
    pub fn new(variables: &'a mut Variables, static_context: &'a context::StaticContext) -> Self {
        Self {
            variables,
            static_context,
            fn_position: ast::Name::new(
                "position".to_string(),
                FN_NAMESPACE.to_string(),
                String::new(),
            ),
            fn_last: ast::Name::new("last".to_string(), FN_NAMESPACE.to_string(), String::new()),
        }
    }

    #[cfg(test)]
    fn convert_expr_single(&mut self, ast: &ast::ExprSingleS) -> error::SpannedResult<ir::ExprS> {
        let bindings = self.expr_single(ast)?;
        Ok(bindings.expr())
    }

    pub(crate) fn convert_xpath(&mut self, ast: &ast::XPath) -> error::SpannedResult<ir::ExprS> {
        let bindings = self.xpath(ast)?;
        Ok(bindings.expr())
    }

    pub fn xpath(&mut self, ast: &ast::XPath) -> error::SpannedResult<Bindings> {
        let context_names = self.variables.push_context();
        // define any external variable names
        let mut ir_names = Vec::new();
        for name in self.static_context.variable_names() {
            ir_names.push(self.variables.new_var_name(name));
        }
        let exprs_bindings = self.expr(&ast.0)?;
        self.variables.pop_context();
        let mut params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
                default: None,
                required: false,
                original_name: None,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
                default: None,
                required: false,
                original_name: None,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
                default: None,
                required: false,
                original_name: None,
            },
        ];
        // add any variables defined in static context as parameters
        for ir_name in ir_names {
            params.push(ir::Param {
                name: ir_name,
                type_: None,
                default: None,
                required: false,
                original_name: None,
            });
        }
        let outer_function_expr = ir::Expr::FunctionDefinition(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(exprs_bindings.expr()),
        });
        let binding = self.variables.new_binding(outer_function_expr, ast.0.span);
        Ok(Bindings::new(binding))
    }

    fn expr_single(&mut self, ast: &ast::ExprSingleS) -> error::SpannedResult<Bindings> {
        let outer_ast = &ast.value;
        let span = ast.span;
        match outer_ast {
            ast::ExprSingle::Path(ast) => self.path_expr(ast),
            ast::ExprSingle::Apply(ast) => self.apply_expr(ast, span),
            ast::ExprSingle::Let(ast) => self.let_expr(ast, span),
            ast::ExprSingle::If(ast) => self.if_expr(ast, span),
            ast::ExprSingle::Binary(ast) => self.binary_expr(ast, span),
            ast::ExprSingle::For(ast) => self.for_expr(ast, span),
            ast::ExprSingle::Quantified(ast) => self.quantified_expr(ast, span),
        }
    }

    fn path_expr(&mut self, ast: &ast::PathExpr) -> error::SpannedResult<Bindings> {
        let first_step = &ast.steps[0];
        let rest_steps = &ast.steps[1..];
        let first_step_bindings = Ok(self.step_expr(first_step)?);
        rest_steps
            .iter()
            .fold(first_step_bindings, |acc, step_expr| {
                let mut step_bindings = acc?;
                let step_atom = step_bindings.atom();
                let context_names = self.variables.push_context();
                let return_bindings = self.step_expr(step_expr)?;
                self.variables.pop_context();
                let expr = ir::Expr::Map(ir::Map {
                    context_names,
                    var_atom: step_atom,
                    return_expr: Box::new(return_bindings.expr()),
                });
                // wrap this in a deduplicate step
                let deduplicate_expr =
                    ir::Expr::Deduplicate(Box::new(Spanned::new(expr, step_expr.span)));

                let binding = self.variables.new_binding(deduplicate_expr, step_expr.span);
                Ok(step_bindings.bind(binding))
            })
    }

    fn step_expr(&mut self, ast: &ast::StepExprS) -> error::SpannedResult<Bindings> {
        let outer_ast = &ast.value;
        let span = ast.span;
        match outer_ast {
            ast::StepExpr::PrimaryExpr(ast) => self.primary_expr(ast),
            ast::StepExpr::PostfixExpr { primary, postfixes } => self.postfixes(primary, postfixes),
            ast::StepExpr::AxisStep(ast) => self.axis_step(ast, span),
        }
    }

    fn primary_expr(&mut self, ast: &ast::PrimaryExprS) -> error::SpannedResult<Bindings> {
        let outer_ast = &ast.value;
        let span = ast.span;
        match outer_ast {
            ast::PrimaryExpr::Literal(ast) => Ok(self.literal(ast, span)?),
            ast::PrimaryExpr::VarRef(ast) => self.variables.var_ref(ast, span),
            ast::PrimaryExpr::Expr(expr) => self.expr_or_empty(expr),
            ast::PrimaryExpr::ContextItem => self.variables.context_item(span),
            ast::PrimaryExpr::InlineFunction(ast) => self.inline_function(ast, span),
            ast::PrimaryExpr::FunctionCall(ast) => self.function_call(ast, span),
            ast::PrimaryExpr::NamedFunctionRef(ast) => self.named_function_ref(ast, span),
            ast::PrimaryExpr::MapConstructor(ast) => self.map_constructor(ast, span),
            ast::PrimaryExpr::ArrayConstructor(ast) => self.array_constructor(ast, span),
            ast::PrimaryExpr::UnaryLookup(ast) => self.unary_lookup(ast, span),
        }
    }

    fn postfixes(
        &mut self,
        primary: &ast::PrimaryExprS,
        postfixes: &[ast::Postfix],
    ) -> error::SpannedResult<Bindings> {
        let primary_bindings = self.primary_expr(primary);
        postfixes.iter().fold(primary_bindings, |acc, postfix| {
            let mut bindings = acc?;
            match postfix {
                ast::Postfix::Predicate(exprs) => {
                    let atom = bindings.atom();
                    let context_names = self.variables.push_context();
                    let return_bindings = self.expr(exprs)?;
                    self.variables.pop_context();
                    let expr = ir::Expr::Filter(ir::Filter {
                        context_names,
                        var_atom: atom,
                        return_expr: Box::new(return_bindings.expr()),
                    });
                    // TODO should use postfix span, not exprs span
                    let binding = self.variables.new_binding(expr, exprs.span);
                    Ok(bindings.bind(binding))
                }
                ast::Postfix::ArgumentList(exprs) => {
                    let atom = bindings.atom();
                    let (arg_bindings, atoms) = self.args(exprs)?;
                    let expr = ir::Expr::FunctionCall(ir::FunctionCall { atom, args: atoms });
                    // TODO should be able to get span for postfix
                    let empty_span = (0..0).into();
                    let binding = self.variables.new_binding(expr, empty_span);
                    Ok(bindings.concat(arg_bindings).bind(binding))
                }
                ast::Postfix::Lookup(key_specifier) => {
                    self.postfix_lookup(key_specifier, &mut bindings)
                }
            }
        })
    }

    fn postfix_lookup(
        &mut self,
        key_specifier: &ast::KeySpecifier,
        bindings: &mut Bindings,
    ) -> error::SpannedResult<Bindings> {
        let span = (0..0).into();

        // the thing we loop over is bound
        let var_atom = bindings.atom();

        // make up an internal name for the loop variable
        let name = self.variables.new_name();
        // access the loop variable
        let mut bindings = bindings.bind(Binding::new(
            name.clone(),
            ir::Expr::Atom(Spanned::new(ir::Atom::Variable(name.clone()), span)),
            span,
        ));

        let atom = bindings.atom();

        let return_bindings = match key_specifier {
            ast::KeySpecifier::NcName(ncname) => {
                let arg_atom =
                    Spanned::new(ir::Atom::Const(ir::Const::String(ncname.clone())), span);
                let arg_bindings = self.atom(arg_atom, span);
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup { atom, arg_atom });
                let binding = self.variables.new_binding(expr, span);
                bindings.bind(binding)
            }
            ast::KeySpecifier::Integer(integer) => {
                let arg_atom =
                    Spanned::new(ir::Atom::Const(ir::Const::Integer(integer.clone())), span);
                let arg_bindings = self.atom(arg_atom, span);
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup { atom, arg_atom });
                let binding = self.variables.new_binding(expr, span);
                bindings.bind(binding)
            }
            ast::KeySpecifier::Expr(expr) => {
                let arg_bindings = self.expr_or_empty(expr)?;
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup { atom, arg_atom });
                let binding = self.variables.new_binding(expr, span);
                bindings.bind(binding)
            }
            ast::KeySpecifier::Star => {
                let expr = ir::Expr::WildcardLookup(ir::WildcardLookup { atom });
                let binding = self.variables.new_binding(expr, span);
                bindings.bind(binding)
            }
        };
        // we wrap the whole thing in a map
        let context_names = self.variables.explicit_context_names(name);
        let expr = ir::Expr::Map(ir::Map {
            context_names,
            var_atom,
            return_expr: Box::new(return_bindings.expr()),
        });
        let binding = self.variables.new_binding(expr, span);
        Ok(bindings.bind(binding))
    }

    fn axis_step(&mut self, ast: &ast::AxisStep, span: Span) -> error::SpannedResult<Bindings> {
        // get the current context
        let mut current_context_bindings = self.variables.context_item(span)?;

        if matches!(ast.axis, ast::Axis::Namespace) {
            return Err(Error::XPST0010.with_ast_span(span));
        }

        let step = xml::Step {
            axis: ast.axis.clone(),
            node_test: ast.node_test.clone(),
        };

        // given the current context item, apply the step
        let expr = ir::Expr::Step(ir::Step {
            step,
            context: current_context_bindings.atom(),
        });

        // create a new binding for the step
        let binding = self.variables.new_binding(expr, span);

        let bindings = Ok(Bindings::new(binding));

        // now apply predicates
        ast.predicates.iter().fold(bindings, |acc, predicate| {
            let mut bindings = acc?;
            let atom = bindings.atom();
            let context_names = self.variables.push_context();
            let return_bindings = self.expr(predicate)?;
            self.variables.pop_context();
            let expr = ir::Expr::Filter(ir::Filter {
                context_names,
                var_atom: atom,
                return_expr: Box::new(return_bindings.expr()),
            });
            let binding = self.variables.new_binding(expr, predicate.span);
            Ok(bindings.bind(binding))
        })
    }

    fn literal(&mut self, ast: &ast::Literal, span: Span) -> error::SpannedResult<Bindings> {
        let atom = match ast {
            ast::Literal::Integer(i) => ir::Atom::Const(ir::Const::Integer(i.clone())),
            ast::Literal::String(s) => ir::Atom::Const(ir::Const::String(s.clone())),
            ast::Literal::Double(d) => ir::Atom::Const(ir::Const::Double(*d)),
            ast::Literal::Decimal(d) => ir::Atom::Const(ir::Const::Decimal(*d)),
        };
        let expr = ir::Expr::Atom(Spanned::new(atom, span));
        let binding = self.variables.new_binding(expr, span);
        Ok(Bindings::new(binding))
    }

    pub fn expr(&mut self, expr: &ast::ExprS) -> error::SpannedResult<Bindings> {
        self.expr_with_span(&expr.value, expr.span)
    }

    fn expr_with_span(&mut self, expr: &ast::Expr, span: Span) -> error::SpannedResult<Bindings> {
        let expr = &expr.0;

        // XXX could this be reduce?
        let first_expr = &expr[0];
        let span_start = span.start;
        let rest_exprs = &expr[1..];
        rest_exprs
            .iter()
            .fold(self.expr_single(first_expr), |acc, expr_single| {
                let mut left_bindings = acc?;
                let mut right_bindings = self.expr_single(expr_single)?;
                let expr = ir::Expr::Binary(ir::Binary {
                    left: left_bindings.atom(),
                    op: ir::BinaryOperator::Comma,
                    right: right_bindings.atom(),
                });
                let span_end = expr_single.span.end;
                let span = (span_start..span_end).into();
                let binding = self.variables.new_binding(expr, span);
                Ok(left_bindings.concat(right_bindings).bind(binding))
            })
    }

    fn expr_or_empty(&mut self, expr: &ast::ExprOrEmptyS) -> error::SpannedResult<Bindings> {
        let span = expr.span;
        if let Some(expr) = &expr.value {
            self.expr_with_span(expr, span)
        } else {
            let expr = ir::Expr::Atom(Spanned::new(
                ir::Atom::Const(ir::Const::EmptySequence),
                span,
            ));
            let binding = self.variables.new_binding(expr, span);
            Ok(Bindings::new(binding))
        }
    }

    fn binary_expr(&mut self, ast: &ast::BinaryExpr, span: Span) -> error::SpannedResult<Bindings> {
        let mut left_bindings = self.path_expr(&ast.left)?;
        let mut right_bindings = self.path_expr(&ast.right)?;
        let op = self.binary_op(ast.operator);
        let expr = ir::Expr::Binary(ir::Binary {
            left: left_bindings.atom(),
            op,
            right: right_bindings.atom(),
        });
        let binding = self.variables.new_binding(expr, span);

        Ok(left_bindings.concat(right_bindings).bind(binding))
    }

    fn binary_op(&mut self, operator: ast::BinaryOperator) -> ir::BinaryOperator {
        operator
    }

    fn apply_expr(&mut self, ast: &ast::ApplyExpr, span: Span) -> error::SpannedResult<Bindings> {
        match &ast.operator {
            ast::ApplyOperator::SimpleMap(path_exprs) => {
                let path_bindings = self.path_expr(&ast.path_expr);
                path_exprs.iter().fold(path_bindings, |acc, path_expr| {
                    let mut path_bindings = acc?;
                    let path_atom = path_bindings.atom();
                    let context_names = self.variables.push_context();
                    let return_bindings = self.path_expr(path_expr)?;
                    self.variables.pop_context();
                    let expr = ir::Expr::Map(ir::Map {
                        context_names,
                        var_atom: path_atom,
                        return_expr: Box::new(return_bindings.expr()),
                    });
                    let binding = self.variables.new_binding(expr, span);
                    Ok(path_bindings.bind(binding))
                })
            }
            ast::ApplyOperator::Unary(ops) => {
                let bindings = self.path_expr(&ast.path_expr);
                ops.iter().rev().fold(bindings, |acc, op| {
                    let mut bindings = acc?;
                    let expr = ir::Expr::Unary(ir::Unary {
                        op: op.clone(),
                        atom: bindings.atom(),
                    });
                    let binding = self.variables.new_binding(expr, span);
                    Ok(bindings.bind(binding))
                })
            }
            ast::ApplyOperator::Cast(single_type) => {
                let xs = Xs::by_name(
                    single_type.name.value.namespace(),
                    single_type.name.value.local_name(),
                );
                if let Some(xs) = xs {
                    if !xs.derives_from(Xs::AnySimpleType) {
                        return Err(Error::XQST0052.with_ast_span(span));
                    }
                    if xs == Xs::Notation || xs == Xs::AnySimpleType || xs == Xs::AnyAtomicType {
                        return Err(Error::XPST0080.with_ast_span(span));
                    }
                    let mut bindings = self.path_expr(&ast.path_expr)?;
                    let expr = ir::Expr::Cast(ir::Cast {
                        atom: bindings.atom(),
                        xs,
                        empty_sequence_allowed: single_type.optional,
                    });
                    let binding = self.variables.new_binding(expr, span);
                    Ok(bindings.bind(binding))
                } else {
                    Err(Error::XQST0052.with_ast_span(span))
                }
            }
            ast::ApplyOperator::Castable(single_type) => {
                let xs = Xs::by_name(
                    single_type.name.value.namespace(),
                    single_type.name.value.local_name(),
                );
                if let Some(xs) = xs {
                    if !xs.derives_from(Xs::AnySimpleType) {
                        return Err(Error::XQST0052.with_ast_span(span));
                    }
                    if xs == Xs::Notation || xs == Xs::AnySimpleType || xs == Xs::AnyAtomicType {
                        return Err(Error::XPST0080.with_ast_span(span));
                    }
                    let mut bindings = self.path_expr(&ast.path_expr)?;
                    let expr = ir::Expr::Castable(ir::Castable {
                        atom: bindings.atom(),
                        xs,
                        empty_sequence_allowed: single_type.optional,
                    });
                    let binding = self.variables.new_binding(expr, span);
                    Ok(bindings.bind(binding))
                } else {
                    Err(Error::XQST0052.with_ast_span(span))
                }
            }
            ast::ApplyOperator::InstanceOf(sequence_type) => {
                let mut bindings = self.path_expr(&ast.path_expr)?;
                let expr = ir::Expr::InstanceOf(ir::InstanceOf {
                    atom: bindings.atom(),
                    sequence_type: sequence_type.clone(),
                });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
            ast::ApplyOperator::Treat(sequence_type) => {
                let mut bindings = self.path_expr(&ast.path_expr)?;
                let expr = ir::Expr::Treat(ir::Treat {
                    atom: bindings.atom(),
                    sequence_type: sequence_type.clone(),
                });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
        }
    }

    fn if_expr(&mut self, ast: &ast::IfExpr, span: Span) -> error::SpannedResult<Bindings> {
        let mut condition_bindings = self.expr(&ast.condition)?;
        let then_bindings = self.expr_single(&ast.then)?;
        let else_bindings = self.expr_single(&ast.else_)?;
        let expr = ir::Expr::If(ir::If {
            condition: condition_bindings.atom(),
            then: Box::new(then_bindings.expr()),
            else_: Box::new(else_bindings.expr()),
        });
        let binding = self.variables.new_binding(expr, span);
        Ok(condition_bindings.bind(binding))
    }

    fn let_expr(&mut self, ast: &ast::LetExpr, span: Span) -> error::SpannedResult<Bindings> {
        let name = self.variables.new_var_name(&ast.var_name.value);
        let var_bindings = self.expr_single(&ast.var_expr)?;
        let return_bindings = self.expr_single(&ast.return_expr)?;
        let expr = ir::Expr::Let(ir::Let {
            name,
            var_expr: Box::new(var_bindings.expr()),
            return_expr: Box::new(return_bindings.expr()),
        });
        Ok(Bindings::new(self.variables.new_binding(expr, span)))
    }

    fn for_expr(&mut self, ast: &ast::ForExpr, span: Span) -> error::SpannedResult<Bindings> {
        let name = self.variables.new_var_name(&ast.var_name.value);
        let mut var_bindings = self.expr_single(&ast.var_expr)?;
        let var_atom = var_bindings.atom();
        let context_names = self.variables.explicit_context_names(name);
        let return_bindings = self.expr_single(&ast.return_expr)?;
        let expr = ir::Expr::Map(ir::Map {
            context_names,
            var_atom,
            return_expr: Box::new(return_bindings.expr()),
        });

        let binding = self.variables.new_binding(expr, span);
        Ok(var_bindings.bind(binding))
    }

    fn quantified_expr(
        &mut self,
        ast: &ast::QuantifiedExpr,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        let name = self.variables.new_var_name(&ast.var_name.value);
        let mut var_bindings = self.expr_single(&ast.var_expr)?;
        let var_atom = var_bindings.atom();

        let context_names = self.variables.explicit_context_names(name);
        let satisfies_bindings = self.expr_single(&ast.satisfies_expr)?;
        let expr = ir::Expr::Quantified(ir::Quantified {
            quantifier: self.quantifier(&ast.quantifier),
            context_names,
            var_atom,
            satisifies_expr: Box::new(satisfies_bindings.expr()),
        });

        let binding = self.variables.new_binding(expr, span);
        Ok(var_bindings.bind(binding))
    }

    fn quantifier(&mut self, quantifier: &ast::Quantifier) -> ir::Quantifier {
        match quantifier {
            ast::Quantifier::Some => ir::Quantifier::Some,
            ast::Quantifier::Every => ir::Quantifier::Every,
        }
    }

    fn inline_function(
        &mut self,
        inline_function: &ast::InlineFunction,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        let params = inline_function
            .params
            .iter()
            .map(|param| self.param(param))
            .collect();
        // wrapper functions (used to simulate placeholder arguments) do not
        // have an absent context, but instead share the context of their
        // environment. Normal inline functions wipe out the context, however.
        if !inline_function.wrapper {
            self.variables.push_absent_context();
        }
        let body_bindings = self.expr_or_empty(&inline_function.body)?;
        if !inline_function.wrapper {
            self.variables.pop_context();
        }
        let expr = ir::Expr::FunctionDefinition(ir::FunctionDefinition {
            params,
            return_type: inline_function.return_type.clone(),
            body: Box::new(body_bindings.expr()),
        });
        let binding = self.variables.new_binding(expr, span);
        Ok(Bindings::new(binding))
    }

    fn param(&mut self, param: &ast::Param) -> ir::Param {
        ir::Param {
            name: self.variables.new_var_name(&param.name),
            type_: param.type_.clone(),
            default: None,
            required: false,
            original_name: None,
        }
    }

    fn function_call(
        &mut self,
        ast: &ast::FunctionCall,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        let arity = ast.arguments.len();
        if arity > u8::MAX as usize {
            return Err(Error::XPDY0130.with_ast_span(span));
        }
        // hardcoded fn:position and fn:last
        // These should work without hardcoding them, but this is faster
        // (until some advanced compiler optimization is implemented)
        // unfortunately this can generate a type error instead of a 'context absent'
        // error in some circumstances, but we can live with that for now as it's
        // much more efficient
        if ast.name.value == self.fn_position {
            if arity != 0 {
                // advice: format!("Either the function name {:?} does not exist, or you are calling it with the wrong number of arguments ({})", ast.name, arity),
                return Err(Error::XPST0017.with_ast_span(span));
            }
            return self.variables.fn_position(span);
        } else if ast.name.value == self.fn_last {
            if arity != 0 {
                // advice: format!("Either the function name {:?} does not exist, or you are calling it with the wrong number of arguments ({})", ast.name, arity),
                return Err(Error::XPST0017.with_ast_span(span));
            }
            return self.variables.fn_last(span);
        }

        // advice: format!("Either the function name {:?} does not exist, or you are calling it with the wrong number of arguments ({})", ast.name, arity),
        let static_function_id = self
            .static_context
            .function_id_by_name(&ast.name.value, arity as u8)
            .ok_or(Error::XPST0017.with_ast_span(span))?;
        // TODO we don't know yet how to get the proper span here
        let empty_span = (0..0).into();
        let mut static_function_ref_bindings =
            self.static_function_ref(static_function_id, empty_span);
        let atom = static_function_ref_bindings.atom();
        let (arg_bindings, atoms) = self.args(&ast.arguments)?;
        let expr = ir::Expr::FunctionCall(ir::FunctionCall { atom, args: atoms });
        let binding = self.variables.new_binding(expr, span);
        Ok(static_function_ref_bindings
            .concat(arg_bindings)
            .bind(binding))
    }

    fn named_function_ref(
        &mut self,
        ast: &ast::NamedFunctionRef,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        // advice: format!("Either the function name {:?} does not exist, or you are calling it with the wrong number of arguments ({})", ast.name, ast.arity),
        let static_function_id = self
            .static_context
            .function_id_by_name(&ast.name.value, ast.arity)
            .ok_or(Error::XPST0017.with_ast_span(span))?;
        Ok(self.static_function_ref(static_function_id, span))
    }

    fn static_function_ref(
        &mut self,
        static_function_id: function::StaticFunctionId,
        span: Span,
    ) -> Bindings {
        let atom = ir::Atom::Const(ir::Const::StaticFunctionReference(
            static_function_id,
            self.variables.current_context_names(),
        ));
        let expr = ir::Expr::Atom(Spanned::new(atom, span));
        let binding = self.variables.new_binding(expr, span);
        Bindings::new(binding)
    }

    fn args(&mut self, args: &[ast::ExprSingleS]) -> error::SpannedResult<(Bindings, Vec<AtomS>)> {
        if args.is_empty() {
            return Ok((Bindings::empty(), vec![]));
        }
        let first = &args[0];
        let rest = &args[1..];
        let mut bindings = self.expr_single(first)?;
        let atoms = vec![bindings.atom()];
        rest.iter()
            .try_fold((bindings, atoms), |(bindings, atoms), arg| {
                let mut arg_bindings = self.expr_single(arg)?;
                let mut atoms = atoms.clone();
                atoms.push(arg_bindings.atom());
                Ok((bindings.concat(arg_bindings), atoms))
            })
    }

    fn unary_lookup(
        &mut self,
        ast: &ast::KeySpecifier,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        let mut bindings = self.variables.context_item(span)?;
        let context_atom = bindings.atom();

        match ast {
            ast::KeySpecifier::NcName(ncname) => {
                let arg_atom =
                    Spanned::new(ir::Atom::Const(ir::Const::String(ncname.clone())), span);
                let arg_bindings = self.atom(arg_atom, span);
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup {
                    atom: context_atom,
                    arg_atom,
                });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
            ast::KeySpecifier::Integer(i) => {
                let arg_atom = Spanned::new(ir::Atom::Const(ir::Const::Integer(i.clone())), span);
                let arg_bindings = self.atom(arg_atom, span);
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup {
                    atom: context_atom,
                    arg_atom,
                });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
            ast::KeySpecifier::Expr(expr) => {
                let arg_bindings = self.expr_or_empty(expr)?;
                let mut bindings = bindings.concat(arg_bindings);
                let arg_atom = bindings.atom();
                let expr = ir::Expr::Lookup(ir::Lookup {
                    atom: context_atom,
                    arg_atom,
                });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
            ast::KeySpecifier::Star => {
                let mut bindings = self.variables.context_item(span)?;
                let context_atom = bindings.atom();
                let expr = ir::Expr::WildcardLookup(ir::WildcardLookup { atom: context_atom });
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
        }
    }

    fn atom(&mut self, atom: AtomS, span: Span) -> Bindings {
        let expr = ir::Expr::Atom(atom);
        let binding = self.variables.new_binding(expr, span);
        Bindings::new(binding)
    }

    fn map_constructor(
        &mut self,
        ast: &ast::MapConstructor,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        let keys = ast
            .entries
            .iter()
            .map(|entry| entry.key.clone())
            .collect::<Vec<_>>();
        let values = ast
            .entries
            .iter()
            .map(|entry| entry.value.clone())
            .collect::<Vec<_>>();
        let (key_bindings, key_atoms) = self.args(&keys)?;
        let (value_bindings, value_atoms) = self.args(&values)?;
        let members = key_atoms.into_iter().zip(value_atoms).collect::<Vec<_>>();
        let expr = ir::Expr::MapConstructor(ir::MapConstructor { members });
        let expr_binding = self.variables.new_binding(expr, span);
        let bindings = key_bindings.concat(value_bindings).bind(expr_binding);
        Ok(bindings)
    }

    fn array_constructor(
        &mut self,
        ast: &ast::ArrayConstructor,
        span: Span,
    ) -> error::SpannedResult<Bindings> {
        match ast {
            ast::ArrayConstructor::Square(expr) => {
                let (bindings, atoms) = self.args(&expr.value.0)?;
                let expr = ir::Expr::ArrayConstructor(ir::ArrayConstructor::Square(atoms));
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
            ast::ArrayConstructor::Curly(expr_or_empty) => {
                let mut bindings = self.expr_or_empty(expr_or_empty)?;
                let expr = ir::Expr::ArrayConstructor(ir::ArrayConstructor::Curly(bindings.atom()));
                let binding = self.variables.new_binding(expr, span);
                Ok(bindings.bind(binding))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    fn convert_expr_single(s: &str) -> error::SpannedResult<ir::ExprS> {
        let ast = ast::ExprSingle::parse(s)?;
        let static_context = context::StaticContext::default();
        let mut variables = Variables::new();
        let mut converter = IrConverter::new(&mut variables, &static_context);
        converter.convert_expr_single(&ast)
    }

    pub(crate) fn convert_xpath(s: &str) -> error::SpannedResult<ir::ExprS> {
        let static_context = context::StaticContext::default();
        let ast = static_context.parse_xpath(s)?;
        let mut variables = Variables::new();
        let mut converter = IrConverter::new(&mut variables, &static_context);
        converter.convert_xpath(&ast)
    }

    #[test]
    fn test_integer() {
        assert_debug_snapshot!(convert_expr_single("1"));
    }

    #[test]
    fn test_add() {
        assert_debug_snapshot!(convert_expr_single("1 + 2"));
    }

    #[test]
    fn test_add2() {
        assert_debug_snapshot!(convert_expr_single("1 + 2 + 3"));
    }

    #[test]
    fn test_if() {
        assert_debug_snapshot!(convert_expr_single("if (1 gt 2) then 1 + 2 else 3 + 4"));
    }

    #[test]
    fn test_comma() {
        assert_debug_snapshot!(convert_xpath("1, 2"));
    }

    #[test]
    fn test_comma2() {
        assert_debug_snapshot!(convert_xpath("1, 2, 3"));
    }

    #[test]
    fn test_empty_sequence() {
        assert_debug_snapshot!(convert_xpath("()"));
    }

    #[test]
    fn test_let_expr() {
        assert_debug_snapshot!(convert_expr_single("let $x := 1 return 2"));
    }

    #[test]
    fn test_let_expr_variable() {
        assert_debug_snapshot!(convert_expr_single("let $x := 1 return $x"));
    }

    #[test]
    fn test_let_expr_with_add() {
        assert_debug_snapshot!(convert_expr_single("let $x := (1 + 2) return $x"));
    }

    #[test]
    fn test_for_expr() {
        assert_debug_snapshot!(convert_expr_single("for $x in 1 return 2"));
    }

    #[test]
    fn test_for_expr2() {
        assert_debug_snapshot!(convert_expr_single("for $x in (1, 2) return $x + 1"));
    }

    #[test]
    fn test_simple_map() {
        assert_debug_snapshot!(convert_expr_single("(1, 2) ! 1"));
    }

    #[test]
    fn test_simple_map_with_context() {
        assert_debug_snapshot!(convert_expr_single("(1, 2) ! (. + 1)"));
    }

    #[test]
    fn test_nested_simple_map_with_context() {
        assert_debug_snapshot!(convert_expr_single("(1, 2) ! (. + 1) ! (. + 2)"));
    }

    #[test]
    fn test_quantified() {
        assert_debug_snapshot!(convert_expr_single("some $x in (1, 2) satisfies $x gt 1"));
    }

    #[test]
    fn test_postfix_filter() {
        assert_debug_snapshot!(convert_expr_single("(1, 2)[. gt 2]"));
    }

    #[test]
    fn test_postfix_filter_nested() {
        assert_debug_snapshot!(convert_expr_single("(1, 2)[. gt 2][. lt 3]"));
    }

    #[test]
    fn test_postfix_index() {
        assert_debug_snapshot!(convert_expr_single("(1, 2)[1]"));
    }

    #[test]
    fn test_function_definition() {
        assert_debug_snapshot!(convert_expr_single("function($x) { $x + 1 }"));
    }

    #[test]
    fn test_function_call() {
        assert_debug_snapshot!(convert_expr_single("function($x) { $x + 1 }(3)"));
    }

    #[test]
    fn test_function_call2() {
        assert_debug_snapshot!(convert_expr_single("function($x) { $x + 1 }(3 + 5)"));
    }

    #[test]
    fn test_static_function_call() {
        assert_debug_snapshot!(convert_expr_single("my_function(5, 2)"));
    }

    #[test]
    fn test_static_function_call2() {
        assert_debug_snapshot!(convert_expr_single("my_function(1 + 2, 3 + 4)"));
    }

    #[test]
    fn test_static_function_call3() {
        assert_debug_snapshot!(convert_expr_single("my_function(1 + 2 + 3, 4 + 5)"));
    }

    #[test]
    fn test_named_function_ref() {
        assert_debug_snapshot!(convert_expr_single("my_function#2"));
    }

    #[test]
    fn test_named_function_ref2() {
        assert_debug_snapshot!(convert_xpath("my_function#2"));
    }

    #[test]
    fn test_path_expr() {
        assert_debug_snapshot!(convert_expr_single("(1, 2) / (. + 1)"));
    }

    #[test]
    fn test_nested_path_expr() {
        assert_debug_snapshot!(convert_expr_single("(1, 2) / (. + 1) / (. + 2)"));
    }

    #[test]
    fn test_single_axis_step() {
        assert_debug_snapshot!(convert_xpath("child::a"));
    }

    #[test]
    fn test_multiple_axis_steps() {
        assert_debug_snapshot!(convert_xpath("child::a/child::b"));
    }

    #[test]
    fn test_axis_step_with_predicates() {
        assert_debug_snapshot!(convert_xpath("child::a[. gt 1]"));
    }

    #[test]
    fn test_absent_context_in_function() {
        assert_debug_snapshot!(convert_xpath("function() { . }"));
    }

    #[test]
    fn test_unknown_static_function_name() {
        assert_debug_snapshot!(convert_expr_single("unknown_function()"));
    }

    #[test]
    fn test_wrong_amount_of_arguments() {
        assert_debug_snapshot!(convert_expr_single("fn:string(1, 2, 3)"));
    }

    #[test]
    fn test_unknown_variable_name() {
        assert_debug_snapshot!(convert_expr_single("$unknown"));
    }

    #[test]
    fn test_unknown_named_function_ref() {
        assert_debug_snapshot!(convert_expr_single("unknown_function#2"));
    }

    #[test]
    fn test_unary() {
        assert_debug_snapshot!(convert_expr_single("-1"));
    }

    #[test]
    fn test_unary_plus() {
        assert_debug_snapshot!(convert_expr_single("+1"));
    }

    #[test]
    fn test_unary_combo() {
        assert_debug_snapshot!(convert_expr_single("-+1"));
    }

    #[test]
    fn test_cast() {
        assert_debug_snapshot!(convert_expr_single("1 cast as xs:string"));
    }

    #[test]
    fn test_cast_question_mark() {
        assert_debug_snapshot!(convert_expr_single("1 cast as xs:string?"));
    }

    #[test]
    fn test_castable() {
        assert_debug_snapshot!(convert_expr_single("1 castable as xs:string"));
    }

    #[test]
    fn test_castable_question_mark() {
        assert_debug_snapshot!(convert_expr_single("1 castable as xs:string?"));
    }

    #[test]
    fn test_cast_unknown_schema_type() {
        assert_debug_snapshot!(convert_expr_single("1 cast as unknown"));
    }

    #[test]
    fn test_cast_non_simple_schema_type() {
        assert_debug_snapshot!(convert_expr_single("1 cast as xs:untyped"));
    }

    #[test]
    fn test_cast_illegal_simple_type() {
        assert_debug_snapshot!(convert_expr_single("1 cast as xs:NOTATION"));
    }

    #[test]
    fn test_instance_of_atomic() {
        assert_debug_snapshot!(convert_expr_single("1 instance of xs:string"));
    }

    #[test]
    fn test_instance_of_kind_test() {
        assert_debug_snapshot!(convert_expr_single("1 instance of node()"));
    }

    #[test]
    fn test_function_call_with_sequence() {
        assert_debug_snapshot!(
            convert_expr_single(
                "compare('a', 'b', ((), 'http://www.w3.org/2005/xpath-functions/collation/codepoint', ()))"));
    }

    #[test]
    fn test_ncname_key_specifier() {
        assert_debug_snapshot!(convert_xpath("? foo"));
    }

    #[test]
    fn test_integer_key_specifier() {
        assert_debug_snapshot!(convert_xpath("? 1"));
    }

    #[test]
    fn test_postfix_lookup() {
        assert_debug_snapshot!(convert_xpath("1 ? foo"));
    }
}
