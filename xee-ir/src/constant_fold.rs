use crate::ir::{
    Atom, AtomS, Binary, BinaryOperator, Const, Expr, ExprS, If, Let, Unary, UnaryOperator,
};
use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::Decimal;
use xee_xpath_ast::span::Span;

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
    if let (
        Atom::Const(left_const),
        Atom::Const(right_const),
    ) = (&binary.left.value, &binary.right.value) {
        match (left_const, right_const) {
            (Const::Integer(l), Const::Integer(r)) => {
                match binary.op {
                    BinaryOperator::Add => {
                        return Expr::Atom(AtomS {
                            value: Atom::Const(Const::Integer(l + r)),
                            span,
                        });
                    }
                    BinaryOperator::Subtract => {
                        return Expr::Atom(AtomS {
                            value: Atom::Const(Const::Integer(l - r)),
                            span,
                        });
                    }
                    BinaryOperator::Multiply => {
                        return Expr::Atom(AtomS {
                            value: Atom::Const(Const::Integer(l * r)),
                            span,
                        });
                    }
                    // Add more integer operations as needed
                    _ => {}
                }
            }
            (Const::String(l), Const::String(r)) => {
                if binary.op == BinaryOperator::Concatenate {
                    return Expr::Atom(AtomS {
                        value: Atom::Const(Const::String(format!("{}{}", l, r))),
                        span,
                    });
                }
            }
            // Add more constant type combinations as needed
            _ => {}
        }
    }
    Expr::Binary(binary.clone())
}

fn fold_unary(unary: &Unary, span: Span) -> Expr {
    if let Atom::Const(const_val) = &unary.atom.value {
        match (const_val, &unary.op) {
            (Const::Integer(val), UnaryOperator::Minus) => {
                return Expr::Atom(AtomS {
                    value: Atom::Const(Const::Integer(-val.clone())),
                    span,
                });
            }
            // Add more unary operations as needed
            _ => {}
        }
    }
    Expr::Unary(unary.clone())
}

fn fold_let(let_expr: &Let, span: Span) -> Expr {
    let var_expr = Box::new(fold_expr(&let_expr.var_expr));
    let return_expr = Box::new(fold_expr(&let_expr.return_expr));
    
    Expr::Let(Let {
        name: let_expr.name.clone(),
        var_expr,
        return_expr,
    })
}

fn fold_if(if_expr: &If, span: Span) -> Expr {
    // If we have a constant condition, we can eliminate the branch
    if let Atom::Const(const_cond) = &if_expr.condition.value {
        match const_cond {
            Const::Integer(val) => {
                if !val.is_zero() {
                    return fold_expr(&if_expr.then).value;
                } else {
                    return fold_expr(&if_expr.else_).value;
                }
            }
            // Add more constant condition types as needed
            _ => {}
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
    use xee_xpath_ast::span::Span;

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
                op: BinaryOperator::Subtract,
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
    fn test_fold_string_concatenation() {
        let expr = ExprS {
            value: Expr::Binary(Binary {
                left: make_string("Hello"),
                op: BinaryOperator::Concatenate,
                right: make_string(" World"),
            }),
            span: dummy_span(),
        };

        let result = fold_expr(&expr);
        
        assert_eq!(
            result.value,
            Expr::Atom(AtomS {
                value: Atom::Const(Const::String("Hello World".to_string())),
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
        
        assert_eq!(
            result.value,
            Expr::Atom(make_int(42))
        );
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
        
        assert_eq!(
            result.value,
            Expr::Atom(make_int(24))
        );
    }
}
