use crate::ir::{
    Atom, AtomS, Binary, BinaryOperator, Const, Expr, ExprS, If, Let, Unary, UnaryOperator,
};
use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::Decimal;
use xee_xpath_ast::ast::Span;

pub fn fold_expr(expr: &ExprS) -> ExprS {
    let span = expr.span;
    let folded = match &expr.value {
        Expr::Atom(atom) => Expr::Atom(atom.clone()),
        Expr::Binary(binary) => fold_binary(binary, span),
        Expr::Unary(unary) => fold_unary(unary, span),
        Expr::Let(let_expr) => fold_let(let_expr, span),
        Expr::If(if_expr) => fold_if(if_expr, span),
        // Pass through other expressions unchanged for now
        _ => expr.value.clone(),
    };
    ExprS {
        value: folded,
        span,
    }
}

fn fold_binary(binary: &Binary, span: Span) -> Expr {
    if let (Atom::Const(left_const), Atom::Const(right_const)) =
        (&binary.left.value, &binary.right.value)
    {
        if let (Const::Integer(l), Const::Integer(r)) = (left_const, right_const) {
            match binary.op {
                BinaryOperator::Add => {
                    return Expr::Atom(AtomS {
                        value: Atom::Const(Const::Integer(l + r)),
                        span,
                    });
                }
                BinaryOperator::Sub => {
                    return Expr::Atom(AtomS {
                        value: Atom::Const(Const::Integer(l - r)),
                        span,
                    });
                }
                BinaryOperator::Mul => {
                    return Expr::Atom(AtomS {
                        value: Atom::Const(Const::Integer(l * r)),
                        span,
                    });
                }
                // Add more integer operations as needed
                _ => {}
            }
        }
    }
    Expr::Binary(binary.clone())
}

fn fold_unary(unary: &Unary, span: Span) -> Expr {
    if let Atom::Const(const_val) = &unary.atom.value {
        if let (Const::Integer(val), UnaryOperator::Minus) = (const_val, &unary.op) {
            return Expr::Atom(AtomS {
                value: Atom::Const(Const::Integer(-val.clone())),
                span,
            });
        }
    }
    Expr::Unary(unary.clone())
}

fn fold_let(let_expr: &Let, span: Span) -> Expr {
    let var_expr = Box::new(fold_expr(&let_expr.var_expr));
    
    // If the var_expr folded to a constant, substitute it in the return expression
    if let Expr::Atom(atom) = &var_expr.value {
        // Create a modified return expression with the variable replaced by the constant
        let modified_return = substitute_var(&let_expr.return_expr, &let_expr.name, atom);
        return fold_expr(&modified_return).value;
    }

    // Otherwise keep the let expression with folded subexpressions
    let return_expr = Box::new(fold_expr(&let_expr.return_expr));
    Expr::Let(Let {
        name: let_expr.name.clone(),
        var_expr,
        return_expr,
    })
}

fn substitute_var(expr: &ExprS, var_name: &Name, replacement: &AtomS) -> ExprS {
    let span = expr.span;
    let value = match &expr.value {
        Expr::Atom(atom) => {
            if let Atom::Variable(name) = &atom.value {
                if name == var_name {
                    return replacement.clone();
                }
            }
            Expr::Atom(atom.clone())
        }
        Expr::Binary(binary) => Expr::Binary(Binary {
            left: substitute_var_atom(&binary.left, var_name, replacement),
            op: binary.op,
            right: substitute_var_atom(&binary.right, var_name, replacement),
        }),
        Expr::If(if_expr) => Expr::If(If {
            condition: substitute_var_atom(&if_expr.condition, var_name, replacement),
            then: Box::new(substitute_var(&if_expr.then, var_name, replacement)),
            else_: Box::new(substitute_var(&if_expr.else_, var_name, replacement)),
        }),
        // Add other cases as needed
        _ => expr.value.clone(),
    };
    ExprS { value, span }
}

fn substitute_var_atom(atom: &AtomS, var_name: &Name, replacement: &AtomS) -> AtomS {
    if let Atom::Variable(name) = &atom.value {
        if name == var_name {
            return replacement.clone();
        }
    }
    atom.clone()
}

fn fold_if(if_expr: &If, span: Span) -> Expr {
    // If we have a constant condition, we can eliminate the branch
    if let Atom::Const(Const::Integer(val)) = &if_expr.condition.value {
        let zero: IBig = 0.into();
        if val != &zero {
            return fold_expr(&if_expr.then).value;
        } else {
            return fold_expr(&if_expr.else_).value;
        }
    }

    Expr::If(If {
        condition: if_expr.condition.clone(),
        then: Box::new(fold_expr(&if_expr.then)),
        else_: Box::new(fold_expr(&if_expr.else_)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xee_interpreter::function::Name;
    use xee_xpath_ast::ast::Span;

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    fn make_int(i: i32) -> AtomS {
        AtomS {
            value: Atom::Const(Const::Integer(IBig::from(i))),
            span: dummy_span(),
        }
    }

    fn make_string(s: &str) -> AtomS {
        AtomS {
            value: Atom::Const(Const::String(s.to_string())),
            span: dummy_span(),
        }
    }

    #[test]
    fn test_fold_binary_add() {
        let expr = ExprS {
            value: Expr::Binary(Binary {
                left: make_int(2),
                op: BinaryOperator::Add,
                right: make_int(3),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        assert_eq!(
            result.value,
            Expr::Atom(AtomS {
                value: Atom::Const(Const::Integer(IBig::from(5))),
                span: dummy_span(),
            })
        );
    }

    #[test]
    fn test_fold_binary_subtract() {
        let expr = ExprS {
            value: Expr::Binary(Binary {
                left: make_int(5),
                op: BinaryOperator::Sub,
                right: make_int(3),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        assert_eq!(
            result.value,
            Expr::Atom(AtomS {
                value: Atom::Const(Const::Integer(IBig::from(2))),
                span: dummy_span(),
            })
        );
    }

    #[test]
    fn test_fold_unary_minus() {
        let expr = ExprS {
            value: Expr::Unary(Unary {
                op: UnaryOperator::Minus,
                atom: make_int(42),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        assert_eq!(
            result.value,
            Expr::Atom(AtomS {
                value: Atom::Const(Const::Integer(IBig::from(-42))),
                span: dummy_span(),
            })
        );
    }

    #[test]
    fn test_fold_if_true_condition() {
        let expr = ExprS {
            value: Expr::If(If {
                condition: make_int(1),
                then: Box::new(ExprS {
                    value: Expr::Atom(make_int(42)),
                    span: dummy_span(),
                }),
                else_: Box::new(ExprS {
                    value: Expr::Atom(make_int(24)),
                    span: dummy_span(),
                }),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        assert_eq!(result.value, Expr::Atom(make_int(42)));
    }

    #[test]
    fn test_fold_if_false_condition() {
        let expr = ExprS {
            value: Expr::If(If {
                condition: make_int(0),
                then: Box::new(ExprS {
                    value: Expr::Atom(make_int(42)),
                    span: dummy_span(),
                }),
                else_: Box::new(ExprS {
                    value: Expr::Atom(make_int(24)),
                    span: dummy_span(),
                }),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        assert_eq!(result.value, Expr::Atom(make_int(24)));
    }

    #[test]
    fn test_fold_let_if_add() {
        // Test folding: let $x := 5 + 3 return if ($x) then 42 else 24
        let name = Name::new("x".to_string());
        let expr = ExprS {
            value: Expr::Let(Let {
                name: name.clone(),
                var_expr: Box::new(ExprS {
                    value: Expr::Binary(Binary {
                        left: make_int(5),
                        op: BinaryOperator::Add,
                        right: make_int(3),
                    }),
                    span: dummy_span(),
                }),
                return_expr: Box::new(ExprS {
                    value: Expr::If(If {
                        condition: AtomS {
                            value: Atom::Variable(name),
                            span: dummy_span(),
                        },
                        then: Box::new(ExprS {
                            value: Expr::Atom(make_int(42)),
                            span: dummy_span(),
                        }),
                        else_: Box::new(ExprS {
                            value: Expr::Atom(make_int(24)),
                            span: dummy_span(),
                        }),
                    }),
                    span: dummy_span(),
                }),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);

        // Should fold to 42 because:
        // 1. 5 + 3 = 8
        // 2. if (8) then 42 else 24 = 42
        assert_eq!(result.value, Expr::Atom(make_int(42)));
    }
}
