use xee_xpath_ast::{ast::Span, span::Spanned};

use crate::{ir, Variables};

/// A binding consists of a unique variable name and an expression.
#[derive(Debug, Clone)]
pub struct Binding {
    name: ir::Name,
    expr: ir::Expr,
    span: Span,
}

impl Binding {
    #[inline]
    pub fn new(name: ir::Name, expr: ir::Expr, span: Span) -> Self {
        Self { name, expr, span }
    }
}

#[derive(Debug, Clone)]
pub struct Bindings {
    bindings: Vec<Binding>,
}

impl Bindings {
    pub fn new(binding: Binding) -> Self {
        Self {
            bindings: vec![binding],
        }
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Create an atom
    /// Takes the last added binding
    /// If it's already an atom, return it, and pops it from the bindings.
    /// If it's not atom, create a variable based on its name and
    /// return that as an atom.
    pub fn atom(&mut self) -> ir::AtomS {
        let last = self.bindings.last().unwrap();
        let (want_pop, atom) = match &last.expr {
            ir::Expr::Atom(atom) => (true, atom.clone()),
            _ => (
                false,
                Spanned::new(ir::Atom::Variable(last.name.clone()), last.span),
            ),
        };
        if want_pop {
            self.bindings.pop();
        }
        atom
    }

    /// Given bindings, return a let expression.
    /// This takes all the bindings and wraps it in a let expression.
    pub fn expr(&self) -> ir::ExprS {
        let last_binding = self.bindings.last().unwrap();
        let bindings = &self.bindings[..self.bindings.len() - 1];
        let expr = last_binding.expr.clone();
        Spanned::new(
            bindings.iter().rev().fold(expr, |expr, binding| {
                ir::Expr::Let(ir::Let {
                    name: binding.name.clone(),
                    var_expr: Box::new(Spanned::new(binding.expr.clone(), binding.span)),
                    return_expr: Box::new(Spanned::new(expr, last_binding.span)),
                })
            }),
            last_binding.span,
        )
    }

    pub fn atom_bindings(mut self) -> (ir::AtomS, Self) {
        let atom = self.atom();
        (atom, self)
    }

    pub fn bind_expr(&self, variables: &mut Variables, expr: ir::ExprS) -> Self {
        let binding = variables.new_binding(expr.value, expr.span);
        self.bind(binding)
    }

    pub fn bind_expr_no_span(&self, variables: &mut Variables, expr: ir::Expr) -> Self {
        let binding = variables.new_binding(expr, (0..0).into());
        self.bind(binding)
    }

    /// Create a new Bindings by adding the existing binding to it
    pub fn bind(&self, binding: Binding) -> Self {
        let mut bindings = self.clone();
        bindings.bindings.push(binding);
        bindings
    }

    /// Concatenate one bindings object with another, creating a new one.
    pub fn concat(&self, bindings: Self) -> Self {
        let mut result = self.clone();
        result.bindings.extend(bindings.bindings);
        result
    }
}
