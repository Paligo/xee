use ahash::{HashMap, HashMapExt};
use xee_interpreter::error::{self, Error};
use xee_xpath_ast::{
    ast::{self, Span},
    span::Spanned,
};

use crate::{ir, Binding, Bindings};

#[derive(Debug)]
enum ContextItem {
    Names(ir::ContextNames),
    IterateNames(ir::ContextNames, ir::Name),
    Absent,
}

#[derive(Debug, Default)]
pub struct Variables {
    counter: usize,
    variables: HashMap<ast::Name, ir::Name>,
    context_scope: Vec<ContextItem>,
}

impl Variables {
    pub fn new() -> Self {
        Self {
            counter: 0,
            variables: HashMap::new(),
            context_scope: Vec::new(),
        }
    }

    pub fn new_binding(&mut self, expr: ir::Expr, span: Span) -> Binding {
        let name = self.new_name();
        Binding::new(name, expr, span)
    }

    pub fn new_binding_no_span(&mut self, expr: ir::Expr) -> Binding {
        let name = self.new_name();
        Binding::new(name, expr, (0..0).into())
    }

    pub fn new_name(&mut self) -> ir::Name {
        let name = format!("v{}", self.counter);
        self.counter += 1;
        ir::Name::new(name)
    }

    pub fn new_var_name(&mut self, name: &ast::Name) -> ir::Name {
        self.variables.get(name).cloned().unwrap_or_else(|| {
            let new_name = self.new_name();
            self.variables.insert(name.clone(), new_name.clone());
            new_name
        })
    }

    pub fn push_context(&mut self) -> ir::ContextNames {
        let names = ir::ContextNames {
            item: self.new_name(),
            position: self.new_name(),
            last: self.new_name(),
        };
        self.context_scope.push(ContextItem::Names(names.clone()));
        names
    }

    pub fn push_iterate_context(&mut self) -> (ir::ContextNames, ir::Name) {
        let names = ir::ContextNames {
            item: self.new_name(),
            position: self.new_name(),
            last: self.new_name(),
        };
        let loop_name = self.new_name();
        self.context_scope
            .push(ContextItem::IterateNames(names.clone(), loop_name.clone()));
        (names, loop_name)
    }

    pub fn push_absent_context(&mut self) {
        self.context_scope.push(ContextItem::Absent);
    }

    pub fn pop_context(&mut self) {
        self.context_scope.pop();
    }

    pub fn explicit_context_names(&mut self, name: ir::Name) -> ir::ContextNames {
        ir::ContextNames {
            item: name,
            position: self.new_name(),
            last: self.new_name(),
        }
    }

    pub fn var_ref(&mut self, name: &ast::Name, span: Span) -> error::SpannedResult<Bindings> {
        let ir_name = self
            .variables
            .get(name)
            .ok_or(Error::XPST0008.with_ast_span(span))?;
        Ok(Bindings::new(Binding::new(
            ir_name.clone(),
            ir::Expr::Atom(Spanned::new(ir::Atom::Variable(ir_name.clone()), span)),
            span,
        )))
    }

    pub fn current_context_names(&self) -> Option<ir::ContextNames> {
        match self.context_scope.last() {
            Some(ContextItem::Names(names) | ContextItem::IterateNames(names, _)) => {
                Some(names.clone())
            }
            Some(ContextItem::Absent) => None,
            None => None,
        }
    }

    pub fn current_iterate_loop_name(&self) -> Option<ir::Name> {
        match self.context_scope.last() {
            Some(ContextItem::IterateNames(_, loop_name)) => Some(loop_name.clone()),
            _ => None,
        }
    }

    // TODO: we're not using the span for error messages
    fn context_name<F>(&mut self, get_name: F, span: Span) -> error::SpannedResult<Bindings>
    where
        F: Fn(&ir::ContextNames) -> ir::Name,
    {
        // TODO: could we get correct psna from ir_name?
        let empty_span: Span = (0..0).into();
        if let Some(context_scope) = self.context_scope.last() {
            match context_scope {
                ContextItem::Names(names) | ContextItem::IterateNames(names, _) => {
                    let ir_name = get_name(names);
                    Ok(Bindings::new(Binding::new(
                        ir_name.clone(),
                        ir::Expr::Atom(Spanned::new(ir::Atom::Variable(ir_name), empty_span)),
                        empty_span,
                    )))
                }
                // we can detect statically that the context is absent if it's in
                // a function definition
                ContextItem::Absent => Err(Error::XPDY0002.with_ast_span(span)),
            }
        } else {
            Err(Error::XPDY0002.with_ast_span(span))
        }
    }

    pub fn context_item(&mut self, span: Span) -> error::SpannedResult<Bindings> {
        self.context_name(|names| names.item.clone(), span)
    }

    pub fn fn_position(&mut self, span: Span) -> error::SpannedResult<Bindings> {
        self.context_name(|names| names.position.clone(), span)
    }

    pub fn fn_last(&mut self, span: Span) -> error::SpannedResult<Bindings> {
        self.context_name(|names| names.last.clone(), span)
    }
}
